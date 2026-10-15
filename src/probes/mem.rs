//! Memory probes: `mem.summary` (RAM and swap, from /proc/meminfo and
//! vm.swappiness), `mem.vm` (virtual memory counters), and the hardware ones
//! for Part I: `mem.numa`, `mem.errors`.

use std::collections::HashMap;

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "mem.summary";
pub(crate) const VM: &str = "mem.vm";
pub(crate) const NUMA: &str = "mem.numa";
pub(crate) const ERRORS: &str = "mem.errors";

/// All sizes in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemSummary {
    pub total: u64,
    pub free: u64,
    /// `MemAvailable`, since kernel 3.14.
    pub available: Option<u64>,
    /// What `free` shows as buff/cache: Buffers + Cached + SReclaimable.
    pub buff_cache: u64,
    /// tmpfs and shared memory, counted inside buff/cache.
    pub shared: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub swappiness: Option<u32>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/meminfo") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let fields = parse_meminfo(&text);
    let get = |k: &str| fields.get(k).copied();

    let (Some(total), Some(free)) = (get("MemTotal"), get("MemFree")) else {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("/proc/meminfo: no MemTotal or MemFree line".to_string());
        return r;
    };
    let summary = MemSummary {
        total,
        free,
        available: get("MemAvailable"),
        buff_cache: get("Buffers").unwrap_or(0)
            + get("Cached").unwrap_or(0)
            + get("SReclaimable").unwrap_or(0),
        shared: get("Shmem").unwrap_or(0),
        swap_total: get("SwapTotal").unwrap_or(0),
        swap_free: get("SwapFree").unwrap_or(0),
        swappiness: ctx
            .read_trimmed("/proc/sys/vm/swappiness")
            .and_then(|s| s.parse().ok()),
    };

    let mut notes = Vec::new();
    if summary.available.is_none() {
        notes.push("no MemAvailable in /proc/meminfo (kernel older than 3.14)".to_string());
    }
    if summary.swappiness.is_none() {
        notes.push("/proc/sys/vm/swappiness: can't read it".to_string());
    }
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(NAME, status, Some(ProbeData::Mem(summary)));
    r.notes = notes;
    r
}

/// Virtual memory counters, since boot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VirtualMemory {
    /// Bytes per page, as the kernel maps them for this process.
    pub page_size: Option<u64>,
    /// Faults served from RAM (a page to map, a copy to make).
    pub minor_faults: Option<u64>,
    /// Faults that had to read the disk.
    pub major_faults: Option<u64>,
    /// Pages read from and written to swap.
    pub swap_in: Option<u64>,
    pub swap_out: Option<u64>,
    /// Transparent huge pages mode: `always`, `madvise` or `never`.
    pub thp: Option<String>,
}

pub(crate) fn vm(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/vmstat") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(VM, e),
    };
    let get = |k: &str| {
        text.lines().find_map(|l| {
            let (key, v) = l.split_once(' ')?;
            (key == k).then(|| v.trim().parse::<u64>().ok()).flatten()
        })
    };
    let (faults, major) = (get("pgfault"), get("pgmajfault"));
    let page_size = ctx.read("/proc/self/smaps").ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("KernelPageSize:"))
            .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
            .map(|kb| kb * 1024)
    });
    // `always [madvise] never`: the mode in brackets is the active one.
    let thp = ctx
        .read_trimmed("/sys/kernel/mm/transparent_hugepage/enabled")
        .and_then(|t| {
            let start = t.find('[')?;
            let end = t[start..].find(']')?;
            Some(t[start + 1..start + end].to_string())
        });
    let data = VirtualMemory {
        page_size,
        minor_faults: faults.zip(major).map(|(f, m)| f.saturating_sub(m)),
        major_faults: major,
        swap_in: get("pswpin"),
        swap_out: get("pswpout"),
        thp,
    };
    ProbeResult::new(VM, Status::Ok, Some(ProbeData::Vm(data)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Numa {
    /// Empty when the kernel has no NUMA support: all RAM is one node.
    pub nodes: Vec<NumaNode>,
    /// Automatic NUMA balancing (`kernel.numa_balancing`) is on.
    pub balancing: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NumaNode {
    pub id: u32,
    /// CPUs of this node: `0-15`.
    pub cpus: String,
    /// Bytes.
    pub total: Option<u64>,
    pub free: Option<u64>,
    /// Pages a process got from the node it wanted, and from another one.
    pub hit: Option<u64>,
    pub miss: Option<u64>,
    /// Relative distance to each node, this one first being 10.
    pub distances: Vec<u32>,
}

pub(crate) fn numa(ctx: &ProbeCtx) -> ProbeResult {
    const DIR: &str = "/sys/devices/system/node";
    let mut ids: Vec<u32> = ctx
        .list(DIR)
        .unwrap_or_default()
        .iter()
        .filter_map(|n| n.strip_prefix("node")?.parse().ok())
        .collect();
    ids.sort_unstable();
    let nodes = ids
        .into_iter()
        .map(|id| {
            let dir = format!("{DIR}/node{id}");
            let info = ctx.read(&format!("{dir}/meminfo")).unwrap_or_default();
            // `Node 0 MemTotal:  16264712 kB`: drop `Node 0`, then parse.
            let info: String = info
                .lines()
                .map(|l| {
                    l.trim_start_matches("Node ")
                        .trim_start_matches(|c: char| c.is_ascii_digit())
                })
                .collect::<Vec<_>>()
                .join("\n");
            let info = parse_meminfo(&info);
            let stat = ctx.read(&format!("{dir}/numastat")).unwrap_or_default();
            let stat = |k: &str| {
                stat.lines().find_map(|l| {
                    let (key, v) = l.split_once(' ')?;
                    (key == k).then(|| v.trim().parse::<u64>().ok()).flatten()
                })
            };
            NumaNode {
                id,
                cpus: ctx
                    .read_trimmed(&format!("{dir}/cpulist"))
                    .unwrap_or_default(),
                total: info.get("MemTotal").copied(),
                free: info.get("MemFree").copied(),
                hit: stat("numa_hit"),
                miss: stat("numa_miss"),
                distances: ctx
                    .read_trimmed(&format!("{dir}/distance"))
                    .unwrap_or_default()
                    .split_whitespace()
                    .filter_map(|d| d.parse().ok())
                    .collect(),
            }
        })
        .collect();
    let data = Numa {
        nodes,
        balancing: ctx
            .read_trimmed("/proc/sys/kernel/numa_balancing")
            .map(|b| b != "0"),
    };
    ProbeResult::new(NUMA, Status::Ok, Some(ProbeData::Numa(data)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemErrors {
    /// Memory controllers that report errors (EDAC); none without ECC.
    pub controllers: Vec<MemController>,
    /// RAM the kernel stopped using after an uncorrectable error, in bytes.
    pub hardware_corrupted: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemController {
    /// `mc0`.
    pub name: String,
    /// Driver's name for it: `Skylake Socket#0 IMC#0`.
    pub model: Option<String>,
    /// Bytes of RAM behind it.
    pub size: Option<u64>,
    /// Memory type and ECC mode of its first DIMM: `Registered-DDR4`, `SECDED`.
    pub mem_type: Option<String>,
    pub ecc_mode: Option<String>,
    /// Errors since boot: fixed by ECC, and not fixable.
    pub corrected: u64,
    pub uncorrected: u64,
}

pub(crate) fn errors(ctx: &ProbeCtx) -> ProbeResult {
    const DIR: &str = "/sys/devices/system/edac/mc";
    let mut names: Vec<String> = ctx
        .list(DIR)
        .unwrap_or_default()
        .into_iter()
        .filter(|n| {
            n.strip_prefix("mc")
                .is_some_and(|i| i.parse::<u32>().is_ok())
        })
        .collect();
    names.sort_by_key(|n| n[2..].parse::<u32>().unwrap_or(0));
    let controllers = names
        .into_iter()
        .map(|name| {
            let dir = format!("{DIR}/{name}");
            let get = |f: &str| ctx.read_trimmed(&format!("{dir}/{f}"));
            let num = |f: &str| get(f).and_then(|v| v.parse::<u64>().ok());
            let dimm = ctx
                .list(&dir)
                .unwrap_or_default()
                .into_iter()
                .find(|d| d.starts_with("dimm") || d.starts_with("rank"));
            let of_dimm = |f: &str| {
                let d = dimm.as_ref()?;
                ctx.read_trimmed(&format!("{dir}/{d}/{f}"))
                    .filter(|v| v != "Unknown")
            };
            MemController {
                model: get("mc_name"),
                size: num("size_mb").map(|mb| mb * 1024 * 1024),
                mem_type: of_dimm("dimm_mem_type"),
                ecc_mode: of_dimm("dimm_edac_mode"),
                corrected: num("ce_count").unwrap_or(0),
                uncorrected: num("ue_count").unwrap_or(0),
                name,
            }
        })
        .collect();
    let data = MemErrors {
        controllers,
        hardware_corrupted: ctx
            .read("/proc/meminfo")
            .ok()
            .and_then(|m| parse_meminfo(&m).get("HardwareCorrupted").copied()),
    };
    ProbeResult::new(ERRORS, Status::Ok, Some(ProbeData::MemErrors(data)))
}

/// `Key:   1234 kB` lines → bytes. Lines without a unit are taken as is.
fn parse_meminfo(text: &str) -> HashMap<&str, u64> {
    text.lines()
        .filter_map(|line| {
            let (key, rest) = line.split_once(':')?;
            let mut parts = rest.split_whitespace();
            let value: u64 = parts.next()?.parse().ok()?;
            let bytes = match parts.next() {
                Some("kB") => value.checked_mul(1024)?,
                Some(_) => return None,
                None => value,
            };
            Some((key.trim(), bytes))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn reads_meminfo_and_swappiness() {
        let r = probe(&fixture("laptop"));
        assert_eq!(r.status, Status::Ok, "{:?}", r.notes);
        let Some(ProbeData::Mem(m)) = r.data else {
            panic!("no data")
        };
        assert_eq!(m.total, 16_264_712 * 1024);
        assert_eq!(m.available, Some(11_018_220 * 1024));
        assert_eq!(m.buff_cache, (412 + 7_431_044 + 389_120) * 1024);
        assert_eq!(m.shared, 618_496 * 1024);
        assert_eq!(m.swappiness, Some(60));
    }

    #[test]
    fn old_kernel_is_partial() {
        let r = probe(&fixture("old-kernel"));
        assert_eq!(r.status, Status::Partial);
        assert_eq!(r.notes.len(), 2, "{:?}", r.notes);
        let Some(ProbeData::Mem(m)) = r.data else {
            panic!("no data")
        };
        assert_eq!(m.available, None);
        assert_eq!(m.swappiness, None);
    }

    #[test]
    fn missing_meminfo_is_unavailable() {
        let r = probe(&fixture("empty"));
        assert_eq!(r.status, Status::Unavailable);
        assert!(r.data.is_none());
        assert_eq!(r.notes, ["/proc/meminfo: not found"]);
    }

    #[test]
    fn garbage_meminfo_is_unavailable() {
        let r = probe(&fixture("garbage"));
        assert_eq!(r.status, Status::Unavailable);
    }

    #[test]
    fn virtual_memory_counters() {
        let Some(ProbeData::Vm(v)) = vm(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(v.page_size, Some(4096));
        assert_eq!(v.minor_faults, Some(812_000_000 - 2_400_000));
        assert_eq!(v.major_faults, Some(2_400_000));
        assert_eq!(v.thp.as_deref(), Some("madvise"));
        assert_eq!(vm(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn numa_nodes() {
        let Some(ProbeData::Numa(n)) = numa(&fixture("server")).data else {
            panic!()
        };
        assert_eq!(n.nodes.len(), 2);
        assert_eq!(n.nodes[1].cpus, "4-7");
        assert_eq!(n.nodes[0].total, Some(65_810_566 * 1024));
        assert_eq!(
            (n.nodes[0].hit, n.nodes[0].miss),
            (Some(500_000_000), Some(25_000_000))
        );
        assert_eq!(n.nodes[1].distances, [20, 10]);
        assert_eq!(n.balancing, Some(true));
        let Some(ProbeData::Numa(n)) = numa(&fixture("empty")).data else {
            panic!()
        };
        assert!(n.nodes.is_empty());
    }

    #[test]
    fn ecc_errors() {
        let Some(ProbeData::MemErrors(e)) = errors(&fixture("server")).data else {
            panic!()
        };
        assert_eq!(e.controllers.len(), 2);
        let mc0 = &e.controllers[0];
        assert_eq!((mc0.corrected, mc0.uncorrected), (3, 0));
        assert_eq!(mc0.mem_type.as_deref(), Some("Registered-DDR4"));
        assert_eq!(mc0.ecc_mode.as_deref(), Some("SECDED"));
        assert_eq!(mc0.size, Some(64 << 30));
        assert_eq!(e.hardware_corrupted, Some(4096));
        let Some(ProbeData::MemErrors(e)) = errors(&fixture("laptop")).data else {
            panic!()
        };
        assert!(e.controllers.is_empty());
        assert_eq!(e.hardware_corrupted, Some(0));
    }

    #[test]
    fn parses_units() {
        let f = parse_meminfo("MemTotal: 2 kB\nHugePages_Total: 3\nBad: x kB\nOdd: 1 MB\n");
        assert_eq!(f.get("MemTotal"), Some(&2048));
        assert_eq!(f.get("HugePages_Total"), Some(&3));
        assert!(!f.contains_key("Bad") && !f.contains_key("Odd"));
    }
}
