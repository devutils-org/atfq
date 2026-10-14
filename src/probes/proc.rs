//! `proc.states`: how many processes are in each state, from /proc/PID/stat.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "proc.states";

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
    fn parses_tricky_names() {
        let s = parse_stat("12 (a) b (c)) Z 7 7 0").unwrap();
        assert_eq!((s.comm.as_str(), s.state, s.ppid), ("a) b (c)", 'Z', 7));
        assert!(parse_stat("12 no-parens S 1").is_none());
        assert!(parse_stat("12 (x)").is_none());
    }
}
