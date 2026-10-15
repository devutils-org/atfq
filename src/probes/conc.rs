//! Concurrency probes: `proc.threads` (who runs the most threads, and the
//! system's ceilings), `proc.limits` (open files and processes allowed), and
//! `ipc.summary` (Unix sockets and shared memory).

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const THREADS: &str = "proc.threads";
pub(crate) const LIMITS: &str = "proc.limits";
pub(crate) const IPC: &str = "ipc.summary";

/// Processes listed by `proc.threads`.
const TOP: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Threads {
    pub processes: u32,
    pub threads: u64,
    /// Processes with the most threads: pid, name, threads.
    pub top: Vec<(u32, String, u64)>,
    /// `kernel.threads-max` and `kernel.pid_max`: every thread takes a PID.
    pub threads_max: Option<u64>,
    pub pid_max: Option<u64>,
}

pub(crate) fn threads(ctx: &ProbeCtx) -> ProbeResult {
    let names = match ctx.list("/proc") {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(THREADS, e),
    };
    let mut all: Vec<(u32, String, u64)> = names
        .iter()
        .filter_map(|n| n.parse::<u32>().ok())
        .filter_map(|pid| {
            let line = ctx.read(&format!("/proc/{pid}/stat")).ok()?;
            let (comm, n) = parse_threads(&line)?;
            Some((pid, comm, n))
        })
        .collect();
    let threads = all.iter().map(|p| p.2).sum();
    let processes = all.len() as u32;
    all.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    all.truncate(TOP);
    let num = |p: &str| ctx.read_trimmed(p).and_then(|v| v.parse().ok());
    let data = Threads {
        processes,
        threads,
        top: all,
        threads_max: num("/proc/sys/kernel/threads-max"),
        pid_max: num("/proc/sys/kernel/pid_max"),
    };
    ProbeResult::new(THREADS, Status::Ok, Some(ProbeData::Threads(data)))
}

/// `/proc/PID/stat` → name and thread count (field 20, the 18th after the
/// name, which may itself hold spaces and parentheses).
fn parse_threads(line: &str) -> Option<(String, u64)> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let comm = line.get(open + 1..close)?.to_string();
    let n = line
        .get(close + 1..)?
        .split_whitespace()
        .nth(17)?
        .parse()
        .ok()?;
    Some((comm, n))
}

/// A resource limit; `None` means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Limit {
    pub soft: Option<u64>,
    pub hard: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Limits {
    /// This process's limits, inherited from the shell that started it.
    pub open_files: Option<Limit>,
    pub processes: Option<Limit>,
    pub stack: Option<Limit>,
    /// Open files on the whole system, and the ceiling (`fs.file-max`).
    pub files_open: Option<u64>,
    pub files_max: Option<u64>,
    /// Highest open-files limit any process may get (`fs.nr_open`).
    pub nr_open: Option<u64>,
}

pub(crate) fn limits(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/self/limits") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(LIMITS, e),
    };
    let find = |name: &str| {
        let line = text.lines().find(|l| l.starts_with(name))?;
        let mut f = line[name.len()..].split_whitespace();
        let value = |v: &str| v.parse::<u64>().ok();
        let soft = f.next()?;
        let hard = f.next()?;
        Some(Limit {
            soft: value(soft),
            hard: value(hard),
        })
    };
    // `1051	0	9223372036854775807`: allocated, unused (always 0), maximum.
    let file_nr: Vec<u64> = ctx
        .read_trimmed("/proc/sys/fs/file-nr")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();
    let data = Limits {
        open_files: find("Max open files"),
        processes: find("Max processes"),
        stack: find("Max stack size"),
        files_open: file_nr
            .first()
            .map(|&a| a - file_nr.get(1).copied().unwrap_or(0)),
        files_max: file_nr.get(2).copied(),
        nr_open: ctx
            .read_trimmed("/proc/sys/fs/nr_open")
            .and_then(|v| v.parse().ok()),
    };
    ProbeResult::new(LIMITS, Status::Ok, Some(ProbeData::Limits(data)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ipc {
    /// Unix domain sockets, and how many of them wait for connections.
    pub unix_sockets: u32,
    pub unix_listening: u32,
    /// Listening sockets with a path, such as `/run/dbus/system_bus_socket`.
    pub listening_paths: Vec<String>,
    /// POSIX shared memory (/dev/shm): bytes used and size.
    pub shm_used: Option<u64>,
    pub shm_size: Option<u64>,
    /// System V shared memory segments and their total size, semaphore
    /// sets and message queues.
    pub sysv_segments: u32,
    pub sysv_bytes: u64,
    pub sysv_semaphores: u32,
    pub sysv_queues: u32,
}

/// Listening socket paths shown by `ipc.summary`.
const SHOWN_PATHS: usize = 4;

pub(crate) fn ipc(ctx: &ProbeCtx) -> ProbeResult {
    let unix = match ctx.read("/proc/net/unix") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(IPC, e),
    };
    let mut data = Ipc {
        unix_sockets: 0,
        unix_listening: 0,
        listening_paths: Vec::new(),
        shm_used: None,
        shm_size: None,
        sysv_segments: 0,
        sysv_bytes: 0,
        sysv_semaphores: 0,
        sysv_queues: 0,
    };
    // `Num RefCount Protocol Flags Type St Inode Path`; flag 00010000 means
    // the socket accepts connections.
    for line in unix.lines().skip(1) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 7 {
            continue;
        }
        data.unix_sockets += 1;
        if f[3] == "00010000" {
            data.unix_listening += 1;
            if let Some(path) = f.get(7).filter(|p| p.starts_with('/')) {
                if !data.listening_paths.iter().any(|p| p == path) {
                    data.listening_paths.push(path.to_string());
                }
            }
        }
    }
    // System services under /run first: they're the ones worth knowing.
    data.listening_paths
        .sort_by_key(|p| (!p.starts_with("/run/"), p.clone()));
    data.listening_paths.truncate(SHOWN_PATHS);
    if let Some(s) = ctx.space("/dev/shm") {
        data.shm_size = Some(s.total);
        data.shm_used = Some(s.total.saturating_sub(s.available));
    }
    let rows = |p: &str| -> Vec<Vec<String>> {
        ctx.read(p)
            .unwrap_or_default()
            .lines()
            .skip(1)
            .map(|l| l.split_whitespace().map(str::to_string).collect())
            .filter(|f: &Vec<String>| !f.is_empty())
            .collect()
    };
    // `key shmid perms size …`
    let shm = rows("/proc/sysvipc/shm");
    data.sysv_segments = shm.len() as u32;
    data.sysv_bytes = shm
        .iter()
        .filter_map(|f| f.get(3)?.parse::<u64>().ok())
        .sum();
    data.sysv_semaphores = rows("/proc/sysvipc/sem").len() as u32;
    data.sysv_queues = rows("/proc/sysvipc/msg").len() as u32;
    ProbeResult::new(IPC, Status::Ok, Some(ProbeData::Ipc(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn thread_counts() {
        let Some(ProbeData::Threads(t)) = threads(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(t.top[0], (4100, "supervisor".to_string(), 12));
        // Truncated stat lines (kernel threads, zombies here) are skipped.
        assert_eq!((t.processes, t.threads), (3, 14));
        assert_eq!(t.threads_max, Some(126_906));
        assert_eq!(t.pid_max, Some(4_194_304));
        assert_eq!(
            parse_threads("7 (a) b) S 1 7 7 0 -1 0 0 0 0 0 1 2 0 0 20 0 9 0"),
            Some(("a) b".to_string(), 9))
        );
        assert_eq!(parse_threads("1 (init) S 0"), None);
    }

    #[test]
    fn process_limits() {
        let Some(ProbeData::Limits(l)) = limits(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(
            l.open_files,
            Some(Limit {
                soft: Some(1024),
                hard: Some(524_288)
            })
        );
        assert_eq!(l.stack.unwrap().hard, None);
        assert_eq!(
            (l.files_open, l.files_max),
            (Some(14_208), Some(9_223_372_036_854_775_807))
        );
        assert_eq!(limits(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn unix_sockets_and_shared_memory() {
        let Some(ProbeData::Ipc(i)) = ipc(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!((i.unix_sockets, i.unix_listening), (5, 3));
        assert_eq!(
            i.listening_paths,
            ["/run/dbus/system_bus_socket", "/run/docker.sock"]
        );
        assert_eq!((i.sysv_segments, i.sysv_bytes), (2, 2_097_152 + 524_288));
        assert_eq!((i.sysv_semaphores, i.sysv_queues), (0, 0));
        assert_eq!(ipc(&fixture("empty")).status, Status::Unavailable);
    }
}
