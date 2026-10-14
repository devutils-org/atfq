//! `proc.states`: how many processes are in each state, from /proc/PID/stat.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "proc.states";
pub(crate) const TOP: &str = "proc.top";
pub(crate) const CGROUP: &str = "proc.cgroup";

/// Clock ticks per second in /proc/PID/stat (USER_HZ): 100 on x86 and ARM.
const TICKS: u64 = 100;

/// Processes listed by `proc.top` in each ranking.
const TOP_N: usize = 5;

/// The lists keep at most this many processes; the counts are complete.
const LIST_MAX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcStates {
    pub total: u32,
    /// State letter (`R`, `S`, `D`, `Z`…) → number of processes.
    pub counts: BTreeMap<String, u32>,
    /// Zombies (`Z`), at most 10.
    pub zombies: Vec<ProcInfo>,
    /// Processes in uninterruptible sleep (`D`), at most 10.
    pub waiting: Vec<ProcInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcInfo {
    pub pid: u32,
    pub comm: String,
    pub ppid: u32,
    /// Name of the parent, if it could be read.
    pub parent: Option<String>,
}

struct Stat {
    comm: String,
    state: char,
    ppid: u32,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let entries = match ctx.list("/proc") {
        Ok(e) => e,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let mut pids: Vec<u32> = entries.iter().filter_map(|n| n.parse().ok()).collect();
    pids.sort_unstable();

    let mut data = ProcStates {
        total: 0,
        counts: BTreeMap::new(),
        zombies: Vec::new(),
        waiting: Vec::new(),
    };
    for pid in pids {
        // A process can exit between the listing and the read: skip it.
        let Some(stat) = read_stat(ctx, pid) else {
            continue;
        };
        data.total += 1;
        *data.counts.entry(stat.state.to_string()).or_default() += 1;
        let list = match stat.state {
            'Z' => &mut data.zombies,
            'D' => &mut data.waiting,
            _ => continue,
        };
        if list.len() < LIST_MAX {
            list.push(ProcInfo {
                pid,
                parent: read_stat(ctx, stat.ppid).map(|p| p.comm),
                comm: stat.comm,
                ppid: stat.ppid,
            });
        }
    }

    if data.total == 0 {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes.push("/proc: no process found".to_string());
        return r;
    }
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Proc(data)))
}

// ----- proc.top --------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TopProcesses {
    /// Biggest resident memory first.
    pub by_memory: Vec<TopEntry>,
    /// Most CPU time used since they started.
    pub by_cpu: Vec<TopEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TopEntry {
    pub pid: u32,
    pub name: String,
    /// Bytes of RAM for `by_memory`, seconds of CPU for `by_cpu`.
    pub amount: u64,
}

pub(crate) fn top(ctx: &ProbeCtx) -> ProbeResult {
    let entries = match ctx.list("/proc") {
        Ok(e) => e,
        Err(e) => return ProbeResult::failed(TOP, e),
    };
    let mut mem = Vec::new();
    let mut cpu = Vec::new();
    for pid in entries.iter().filter_map(|n| n.parse::<u32>().ok()) {
        let Ok(line) = ctx.read(&format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(stat) = parse_stat(&line) else {
            continue;
        };
        let ticks = line
            .rfind(')')
            .and_then(|i| line.get(i + 1..))
            .map(|rest| {
                let f: Vec<&str> = rest.split_whitespace().collect();
                // utime and stime are fields 14 and 15; `rest` starts at 3.
                let n = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                n(11) + n(12)
            })
            .unwrap_or(0);
        cpu.push(TopEntry {
            pid,
            name: stat.comm.clone(),
            amount: ticks / TICKS,
        });
        // Kernel threads have no VmRSS line: they aren't in the memory list.
        let rss = ctx.read(&format!("/proc/{pid}/status")).ok().and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmRSS:"))
                .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
        });
        if let Some(kb) = rss {
            mem.push(TopEntry {
                pid,
                name: stat.comm,
                amount: kb * 1024,
            });
        }
    }
    if cpu.is_empty() {
        let mut r = ProbeResult::new(TOP, Status::Unavailable, None);
        r.notes.push("/proc: no process found".to_string());
        return r;
    }
    for list in [&mut mem, &mut cpu] {
        list.sort_by(|a, b| b.amount.cmp(&a.amount).then(a.pid.cmp(&b.pid)));
        list.truncate(TOP_N);
    }
    ProbeResult::new(
        TOP,
        Status::Ok,
        Some(ProbeData::Top(TopProcesses {
            by_memory: mem,
            by_cpu: cpu,
        })),
    )
}

// ----- proc.cgroup -----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CgroupInfo {
    /// 2 for the unified hierarchy, 1 for the legacy one.
    pub version: u8,
    /// Where this process (idk, so your session) sits.
    pub path: String,
    /// Controllers available at the root (v2).
    pub controllers: Vec<String>,
    /// The tightest memory limit on the way up, and where it is set.
    pub memory_limit: Option<(u64, String)>,
}

pub(crate) fn cgroup(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/self/cgroup") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(CGROUP, e),
    };
    let v2 = ctx.read("/sys/fs/cgroup/cgroup.controllers").ok();
    let path = text
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .or_else(|| text.lines().next().and_then(|l| l.splitn(3, ':').nth(2)))
        .unwrap_or("/")
        .to_string();
    let mut memory_limit: Option<(u64, String)> = None;
    if v2.is_some() {
        // Walk up from our cgroup to the root; the smallest limit wins.
        // `dir` is "" for the root.
        let mut dir = path.trim_end_matches('/').to_string();
        loop {
            let file = format!("/sys/fs/cgroup{dir}/memory.max");
            if let Some(n) = ctx.read_trimmed(&file).and_then(|v| v.parse::<u64>().ok()) {
                if memory_limit.as_ref().is_none_or(|(m, _)| n < *m) {
                    let at = if dir.is_empty() { "/" } else { &dir };
                    memory_limit = Some((n, at.to_string()));
                }
            }
            if dir.is_empty() {
                break;
            }
            dir.truncate(dir.rfind('/').unwrap_or(0));
        }
    }
    let data = CgroupInfo {
        version: if v2.is_some() { 2 } else { 1 },
        path,
        controllers: v2
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        memory_limit,
    };
    ProbeResult::new(CGROUP, Status::Ok, Some(ProbeData::Cgroup(data)))
}

fn read_stat(ctx: &ProbeCtx, pid: u32) -> Option<Stat> {
    parse_stat(&ctx.read(&format!("/proc/{pid}/stat")).ok()?)
}

/// `1234 (some name) S 1 ...`. The name can hold spaces and parentheses,
/// so it ends at the last `)`.
fn parse_stat(line: &str) -> Option<Stat> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let comm = line.get(open + 1..close)?.to_string();
    let mut rest = line.get(close + 1..)?.split_whitespace();
    let state = rest.next()?.chars().next()?;
    let ppid = rest.next()?.parse().ok()?;
    Some(Stat { comm, state, ppid })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn states(name: &str) -> ProcStates {
        match probe(&fixture(name)).data {
            Some(ProbeData::Proc(p)) => p,
            other => panic!("no data: {other:?}"),
        }
    }

    #[test]
    fn counts_states_and_lists_zombies() {
        let p = states("laptop");
        assert_eq!(p.total, 8);
        let counts: Vec<(&str, u32)> = p.counts.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(counts, [("D", 1), ("I", 1), ("R", 1), ("S", 3), ("Z", 2)]);
        assert_eq!(
            p.zombies[0],
            ProcInfo {
                pid: 4242,
                comm: "worker (old)".into(),
                ppid: 4100,
                parent: Some("supervisor".into()),
            }
        );
        // Its parent is gone: no name.
        assert_eq!(p.zombies[1].parent, None);
        assert_eq!(p.waiting[0].comm, "rsync");
    }

    #[test]
    fn skips_unreadable_and_broken_entries() {
        // `garbage` has a pid dir without stat and one with a broken stat.
        let r = probe(&fixture("garbage"));
        assert_eq!(r.status, Status::Ok);
        let Some(ProbeData::Proc(p)) = r.data else {
            panic!()
        };
        assert_eq!(p.total, 1);
    }

    #[test]
    fn missing_proc_is_unavailable() {
        let r = probe(&fixture("empty"));
        assert_eq!(r.status, Status::Unavailable);
        assert_eq!(r.notes, ["/proc: not found"]);
    }

    #[test]
    fn top_processes() {
        let Some(ProbeData::Top(t)) = top(&fixture("laptop")).data else {
            panic!()
        };
        let mem: Vec<&str> = t.by_memory.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(mem, ["supervisor", "rsync", "systemd"]);
        assert_eq!(t.by_memory[0].amount, 812_000 * 1024);
        assert_eq!(t.by_cpu[0].name, "rsync");
        assert_eq!(t.by_cpu[0].amount, 95);
        assert_eq!(top(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn cgroup_v2_with_a_limit_up_the_tree() {
        let Some(ProbeData::Cgroup(c)) = cgroup(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(c.version, 2);
        assert_eq!(c.path, "/user.slice/user-1000.slice/session-2.scope");
        assert_eq!(c.controllers, ["cpu", "memory", "pids"]);
        assert_eq!(
            c.memory_limit,
            Some((
                8 * 1024 * 1024 * 1024,
                "/user.slice/user-1000.slice".to_string()
            ))
        );
        assert_eq!(cgroup(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn parses_tricky_names() {
        let s = parse_stat("12 (a) b (c)) Z 7 7 0").unwrap();
        assert_eq!((s.comm.as_str(), s.state, s.ppid), ("a) b (c)", 'Z', 7));
        assert!(parse_stat("12 no-parens S 1").is_none());
        assert!(parse_stat("12 (x)").is_none());
    }
}
