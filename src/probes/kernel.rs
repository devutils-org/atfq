//! Kernel probes: `kernel.sysctl` (a few telling settings, and the ones set
//! in /etc) and `kernel.modules` (loaded modules and taint flags).

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const SYSCTL: &str = "kernel.sysctl";
pub(crate) const MODULES: &str = "kernel.modules";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

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
    fn missing_files() {
        assert_eq!(sysctl(&fixture("empty")).status, Status::Unavailable);
        assert_eq!(modules(&fixture("empty")).status, Status::Unavailable);
    }
}
