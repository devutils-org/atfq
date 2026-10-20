//! Kernel probes: `kernel.sysctl` (a few telling settings, and the ones set
//! in /etc), `kernel.modules` (loaded modules and taint flags) and
//! `kernel.tracing` (what decides whether a tool may watch the kernel).

use serde::Serialize;

use super::conc::Limit;
use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const SYSCTL: &str = "kernel.sysctl";
pub(crate) const MODULES: &str = "kernel.modules";
pub(crate) const OVERVIEW: &str = "kernel.overview";
pub(crate) const IRQ: &str = "kernel.irq";
pub(crate) const TRACING: &str = "kernel.tracing";

/// Interrupt sources listed by `kernel.irq`.
const IRQ_TOP: usize = 6;

/// Settings worth showing, with what they mean in a few words.
const WATCHED: &[(&str, &str)] = &[
    ("vm.swappiness", "how willing the kernel is to swap"),
    (
        "vm.overcommit_memory",
        "0 guess, 1 always say yes, 2 strict limit",
    ),
    ("kernel.dmesg_restrict", "1: reading dmesg needs root"),
    (
        "kernel.yama.ptrace_scope",
        "0 any, 1 own children, 2 root, 3 none",
    ),
    ("net.ipv4.ip_forward", "1: this machine routes packets"),
    ("kernel.sysrq", "0 off, 1 all, other: some Alt+SysRq keys"),
    (
        "fs.inotify.max_user_watches",
        "files a user can watch for changes",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sysctl {
    pub values: Vec<SysctlValue>,
    /// Settings from /etc/sysctl.conf and /etc/sysctl.d/*.conf.
    pub configured: Vec<SysctlSetting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SysctlValue {
    pub name: String,
    pub value: String,
    pub meaning: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SysctlSetting {
    pub file: String,
    pub name: String,
    pub value: String,
}

/// `vm.swappiness` → `/proc/sys/vm/swappiness`.
fn proc_path(name: &str) -> String {
    format!("/proc/sys/{}", name.replace('.', "/"))
}

pub(crate) fn sysctl(ctx: &ProbeCtx) -> ProbeResult {
    if let Err(e) = ctx.list("/proc/sys") {
        return ProbeResult::failed(SYSCTL, e);
    }
    let values = WATCHED
        .iter()
        .filter_map(|(name, meaning)| {
            let value = ctx.read_trimmed(&proc_path(name))?;
            Some(SysctlValue {
                name: name.to_string(),
                value: value.split_whitespace().collect::<Vec<_>>().join(" "),
                meaning,
            })
        })
        .collect();

    let mut files = vec!["/etc/sysctl.conf".to_string()];
    files.extend(
        ctx.list("/etc/sysctl.d")
            .unwrap_or_default()
            .into_iter()
            .filter(|f| f.ends_with(".conf"))
            .map(|f| format!("/etc/sysctl.d/{f}")),
    );
    let mut configured = Vec::new();
    for file in files {
        // A link to /dev/null disables a file of the same name in /usr/lib.
        if ctx.read_link(&file).as_deref() == Some("/dev/null") {
            continue;
        }
        for line in ctx.read(&file).unwrap_or_default().lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                configured.push(SysctlSetting {
                    file: file.clone(),
                    name: k.trim().trim_start_matches('-').replace('/', "."),
                    value: v.trim().to_string(),
                });
            }
        }
    }
    ProbeResult::new(
        SYSCTL,
        Status::Ok,
        Some(ProbeData::Sysctl(Sysctl { values, configured })),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Modules {
    pub loaded: u32,
    /// The biggest modules, at most 5: name and size in bytes.
    pub largest: Vec<(String, u64)>,
    /// `/proc/sys/kernel/tainted`: 0 means clean.
    pub tainted: Option<u64>,
    /// One letter and its meaning per taint bit set.
    pub taint_flags: Vec<(char, String)>,
}

/// Documentation/admin-guide/tainted-kernels.rst, bit by bit.
const TAINTS: &[(char, &str)] = &[
    ('P', "proprietary module loaded"),
    ('F', "module force-loaded"),
    ('S', "CPU out of specification"),
    ('R', "module force-unloaded"),
    ('M', "machine check error reported by the CPU"),
    ('B', "bad memory page found"),
    ('U', "taint requested by a user program"),
    ('D', "the kernel died recently (oops or BUG)"),
    ('A', "ACPI table overridden"),
    ('W', "the kernel issued a warning"),
    ('C', "staging (experimental) driver loaded"),
    ('I', "firmware bug workaround applied"),
    ('O', "out-of-tree module loaded"),
    ('E', "unsigned module loaded"),
    ('L', "soft lockup occurred"),
    ('K', "kernel live-patched"),
    ('X', "auxiliary taint (set by distros)"),
    ('T', "built with struct randomization"),
    ('N', "in-kernel test run"),
];

pub(crate) fn modules(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/modules") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(MODULES, e),
    };
    let mut all: Vec<(String, u64)> = text
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            Some((f.next()?.to_string(), f.next()?.parse().ok()?))
        })
        .collect();
    let loaded = all.len() as u32;
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    all.truncate(5);

    let tainted = ctx
        .read_trimmed("/proc/sys/kernel/tainted")
        .and_then(|t| t.parse::<u64>().ok());
    let taint_flags = (0..64)
        .filter(|bit| tainted.is_some_and(|t| t & (1 << bit) != 0))
        .map(|bit| match TAINTS.get(bit) {
            Some((c, what)) => (*c, what.to_string()),
            None => ('?', format!("bit {bit}")),
        })
        .collect();
    ProbeResult::new(
        MODULES,
        Status::Ok,
        Some(ProbeData::Modules(Modules {
            loaded,
            largest: all,
            tainted,
            taint_flags,
        })),
    )
}

// ----- kernel.overview -------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KernelOverview {
    pub release: Option<String>,
    /// `#1 SMP PREEMPT_DYNAMIC Mon Oct 12 05:39:36 UTC 2026`: build number,
    /// options and date, from /proc/version.
    pub build: Option<String>,
    /// `gcc 14.2.0` or `clang 22.1.3`.
    pub compiler: Option<String>,
    pub uptime_secs: Option<u64>,
    /// Since boot.
    pub context_switches: Option<u64>,
    pub processes_started: Option<u64>,
    pub interrupts: Option<u64>,
    /// Right now: tasks running or ready, and tasks blocked on I/O.
    pub running: Option<u64>,
    pub blocked: Option<u64>,
    /// Disk filesystems this kernel can mount now (built in or loaded).
    pub filesystems: Vec<String>,
}

pub(crate) fn overview(ctx: &ProbeCtx) -> ProbeResult {
    let stat = match ctx.read("/proc/stat") {
        Ok(s) => s,
        Err(e) => return ProbeResult::failed(OVERVIEW, e),
    };
    let field = |key: &str| {
        stat.lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
    };
    let version = ctx.read_trimmed("/proc/version").unwrap_or_default();
    let compiler = compiler(&version);
    let data = KernelOverview {
        release: ctx.read_trimmed("/proc/sys/kernel/osrelease"),
        build: version
            .rfind(") #")
            .map(|i| version[i + 2..].trim().to_string())
            .filter(|b| !b.is_empty()),
        compiler,
        uptime_secs: ctx
            .read_trimmed("/proc/uptime")
            .and_then(|u| u.split('.').next().and_then(|s| s.parse().ok())),
        context_switches: field("ctxt "),
        processes_started: field("processes "),
        interrupts: field("intr "),
        running: field("procs_running "),
        blocked: field("procs_blocked "),
        filesystems: ctx
            .read("/proc/filesystems")
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.starts_with("nodev"))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
    };
    ProbeResult::new(OVERVIEW, Status::Ok, Some(ProbeData::Kernel(data)))
}

/// The compiler in /proc/version, from the second group in parentheses:
/// `(gcc-12 (Debian 12.2.0-14) 12.2.0, GNU ld …)` (Debian, Ubuntu),
/// `(gcc (GCC) 14.2.1 20240912 (Red Hat 14.2.1-3), …)` (Fedora),
/// `(ClangBuiltLinux clang version 22.1.3 (…), …)`. Gives `gcc 12.2.0`.
fn compiler(version: &str) -> Option<String> {
    // Skip "Linux version X (builder@host) " to the group that follows.
    let after_builder = version.split_once(") (")?.1;
    let group = after_builder.split(", ").next()?;
    let name = if group.contains("clang") {
        "clang"
    } else if group.contains("gcc") {
        "gcc"
    } else {
        return None;
    };
    let is_version =
        |t: &&str| t.contains('.') && t.chars().all(|c| c.is_ascii_digit() || c == '.');
    let v = group.split_whitespace().find(is_version)?;
    Some(format!("{name} {v}"))
}

// ----- kernel.irq ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Interrupts {
    pub cpus: u32,
    /// Busiest hardware and inter-processor interrupts: name and count.
    pub top: Vec<(String, u64)>,
    /// Software interrupts (deferred kernel work), by kind, busiest first.
    pub softirqs: Vec<(String, u64)>,
}

pub(crate) fn irq(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/interrupts") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(IRQ, e),
    };
    let mut lines = text.lines();
    let cpus = lines.next().map_or(0, |h| {
        h.split_whitespace()
            .filter(|c| c.starts_with("CPU"))
            .count()
    });
    let mut top: Vec<(String, u64)> = Vec::new();
    for line in lines {
        let mut f = line.split_whitespace();
        let Some(label) = f.next().map(|l| l.trim_end_matches(':')) else {
            continue;
        };
        let rest: Vec<&str> = f.collect();
        let counts: Vec<u64> = rest
            .iter()
            .take(cpus)
            .map_while(|v| v.parse().ok())
            .collect();
        let total: u64 = counts.iter().sum();
        let desc = &rest[counts.len()..];
        // Numbered lines end with the device; named ones (LOC, IPI0…) with
        // a description.
        let name = if label.chars().all(|c| c.is_ascii_digit()) {
            desc.last().map_or(label.to_string(), |d| d.to_string())
        } else if desc.is_empty() {
            label.to_string()
        } else {
            format!("{label} {}", desc.join(" "))
        };
        if total > 0 {
            top.push((name, total));
        }
    }
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(IRQ_TOP);
    let mut softirqs: Vec<(String, u64)> = ctx
        .read("/proc/softirqs")
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter_map(|l| {
            let (name, counts) = l.split_once(':')?;
            let total = counts
                .split_whitespace()
                .filter_map(|v| v.parse::<u64>().ok())
                .sum();
            Some((name.trim().to_string(), total))
        })
        .filter(|(_, n)| *n > 0)
        .collect();
    softirqs.sort_by_key(|s| std::cmp::Reverse(s.1));
    softirqs.truncate(IRQ_TOP);
    ProbeResult::new(
        IRQ,
        Status::Ok,
        Some(ProbeData::Irq(Interrupts {
            cpus: cpus as u32,
            top,
            softirqs,
        })),
    )
}

/// What a perf, eBPF or tracing tool needs from the kernel, read as an
/// ordinary user: the settings that refuse it, and what this process holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tracing {
    /// `kernel.perf_event_paranoid`: -1 everything, 0 no raw tracepoints,
    /// 1 no CPU-wide events, 2 own processes only; above 2, distro patches
    /// that refuse perf entirely without `CAP_PERFMON`.
    pub perf_event_paranoid: Option<i32>,
    /// `kernel.kptr_restrict`: 0 kernel addresses shown, 1 hidden without
    /// `CAP_SYSLOG`, 2 hidden from everyone.
    pub kptr_restrict: Option<u8>,
    /// `kernel.unprivileged_bpf_disabled`: 0 allowed, 1 or 2 refused without
    /// `CAP_BPF`.
    pub unprivileged_bpf_disabled: Option<u8>,
    /// Kernel lockdown mode: none, integrity or confidentiality.
    pub lockdown: Option<String>,
    /// /sys/kernel/btf/vmlinux: the kernel's own type information, which
    /// portable eBPF programs need.
    pub btf: bool,
    /// Locked-memory limit of this process: older kernels charge eBPF maps
    /// to it.
    pub memlock: Option<Limit>,
    /// Whether this process holds `CAP_PERFMON` and `CAP_BPF` (effective).
    pub cap_perfmon: Option<bool>,
    pub cap_bpf: Option<bool>,
}

/// `CAP_PERFMON` and `CAP_BPF`, as bits of `CapEff` in /proc/self/status.
const CAP_PERFMON_BIT: u32 = 38;
const CAP_BPF_BIT: u32 = 39;

pub(crate) fn tracing(ctx: &ProbeCtx) -> ProbeResult {
    if let Err(e) = ctx.list("/proc/sys/kernel") {
        return ProbeResult::failed(TRACING, e);
    }
    let number = |name: &str| ctx.read_trimmed(&format!("/proc/sys/kernel/{name}"));
    // `none [integrity] confidentiality`: the active mode is in brackets.
    let lockdown = ctx
        .read_trimmed("/sys/kernel/security/lockdown")
        .and_then(|l| Some(l.split('[').nth(1)?.split(']').next()?.to_string()));
    let memlock = ctx.read("/proc/self/limits").ok().and_then(|text| {
        let line = text.lines().find(|l| l.starts_with("Max locked memory"))?;
        let mut f = line["Max locked memory".len()..].split_whitespace();
        let (soft, hard) = (f.next()?, f.next()?);
        Some(Limit {
            soft: soft.parse().ok(),
            hard: hard.parse().ok(),
        })
    });
    let caps = ctx.read("/proc/self/status").ok().and_then(|status| {
        let hex = status.lines().find_map(|l| l.strip_prefix("CapEff:"))?;
        u64::from_str_radix(hex.trim(), 16).ok()
    });
    let data = Tracing {
        perf_event_paranoid: number("perf_event_paranoid").and_then(|v| v.parse().ok()),
        kptr_restrict: number("kptr_restrict").and_then(|v| v.parse().ok()),
        unprivileged_bpf_disabled: number("unprivileged_bpf_disabled").and_then(|v| v.parse().ok()),
        lockdown,
        btf: ctx.exists("/sys/kernel/btf/vmlinux"),
        memlock,
        cap_perfmon: caps.map(|c| c >> CAP_PERFMON_BIT & 1 == 1),
        cap_bpf: caps.map(|c| c >> CAP_BPF_BIT & 1 == 1),
    };
    ProbeResult::new(TRACING, Status::Ok, Some(ProbeData::Tracing(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn tracing_settings_and_capabilities() {
        let Some(ProbeData::Tracing(t)) = tracing(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(t.perf_event_paranoid, Some(2));
        assert_eq!(t.kptr_restrict, Some(1));
        assert_eq!(t.unprivileged_bpf_disabled, Some(2));
        assert_eq!(t.lockdown.as_deref(), Some("none"));
        assert!(t.btf);
        assert_eq!(
            t.memlock,
            Some(Limit {
                soft: Some(8_388_608),
                hard: Some(8_388_608)
            })
        );
        // the fixture's status has no capability lines
        assert_eq!((t.cap_perfmon, t.cap_bpf), (None, None));
        assert_eq!(tracing(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn reads_watched_and_configured_settings() {
        let Some(ProbeData::Sysctl(s)) = sysctl(&fixture("laptop")).data else {
            panic!()
        };
        let names: Vec<&str> = s.values.iter().map(|v| v.name.as_str()).collect();
        // ptrace_scope is missing in the fixture (no Yama): left out.
        assert_eq!(
            names,
            [
                "vm.swappiness",
                "vm.overcommit_memory",
                "kernel.dmesg_restrict",
                "net.ipv4.ip_forward",
                "kernel.sysrq",
                "fs.inotify.max_user_watches"
            ]
        );
        let set: Vec<(&str, &str)> = s
            .configured
            .iter()
            .map(|c| (c.name.as_str(), c.value.as_str()))
            .collect();
        assert_eq!(
            set,
            [
                ("vm.swappiness", "10"),
                ("fs.inotify.max_user_watches", "524288")
            ]
        );
        assert_eq!(s.configured[0].file, "/etc/sysctl.d/90-swap.conf");
    }

    #[test]
    fn modules_and_taint() {
        let Some(ProbeData::Modules(m)) = modules(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(m.loaded, 4);
        assert_eq!(m.largest[0].0, "nvidia");
        assert_eq!(m.tainted, Some(4097));
        let letters: String = m.taint_flags.iter().map(|(c, _)| c).collect();
        assert_eq!(letters, "PO");
    }

    #[test]
    fn overview_counts() {
        let Some(ProbeData::Kernel(k)) = overview(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(k.compiler.as_deref(), Some("gcc 14.2.0"));
        assert_eq!(
            k.build.as_deref(),
            Some("#1 SMP PREEMPT_DYNAMIC Debian 6.12.48-1 (2025-09-20)")
        );
        assert_eq!(k.context_switches, Some(912_345_678));
        assert_eq!(k.processes_started, Some(1_204_311));
        assert_eq!((k.running, k.blocked), (Some(3), Some(1)));
        assert_eq!(k.filesystems, ["ext4", "vfat", "btrfs"]);
        assert_eq!(overview(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn compilers_of_real_distros() {
        let cases = [
            ("Linux version 6.1.0-18-amd64 (debian-kernel@lists.debian.org) (gcc-12 (Debian 12.2.0-14) 12.2.0, GNU ld (GNU Binutils for Debian) 2.40) #1 SMP PREEMPT_DYNAMIC Debian 6.1.76-1 (2024-02-01)", "gcc 12.2.0"),
            ("Linux version 6.11.4-301.fc41.x86_64 (mockbuild@a1d6a7e1b5b8) (gcc (GCC) 14.2.1 20240912 (Red Hat 14.2.1-3), GNU ld version 2.43.1-2.fc41) #1 SMP PREEMPT_DYNAMIC Sun Oct 20 15:02:33 UTC 2024", "gcc 14.2.1"),
            ("Linux version 6.8.0-45-generic (buildd@lcy02-amd64-115) (x86_64-linux-gnu-gcc-13 (Ubuntu 13.2.0-23ubuntu4) 13.2.0, GNU ld (GNU Binutils for Ubuntu) 2.42) #45-Ubuntu SMP PREEMPT_DYNAMIC Fri Aug 30 12:02:04 UTC 2024", "gcc 13.2.0"),
            ("Linux version 7.2.9-orbstack (orbstack@builder) (ClangBuiltLinux clang version 22.1.3 (https://github.com/llvm/llvm-project.git e98466), ClangBuiltLinux LLD 22.1.3) #1 SMP PREEMPT Mon Oct 12 05:39:36 UTC 2026", "clang 22.1.3"),
        ];
        for (v, want) in cases {
            assert_eq!(compiler(v).as_deref(), Some(want), "{v}");
        }
        assert_eq!(compiler("garbage"), None);
    }

    #[test]
    fn interrupts_by_source() {
        let Some(ProbeData::Irq(i)) = irq(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(i.cpus, 2);
        assert_eq!(i.top[0], ("LOC Local timer interrupts".to_string(), 900));
        assert_eq!(i.top[1], ("iwlwifi".to_string(), 300));
        assert_eq!(i.softirqs[0], ("TIMER".to_string(), 70));
        assert_eq!(irq(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn missing_files() {
        assert_eq!(sysctl(&fixture("empty")).status, Status::Unavailable);
        assert_eq!(modules(&fixture("empty")).status, Status::Unavailable);
    }
}
