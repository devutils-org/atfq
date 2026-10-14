//! Live probes: what this machine says.
//!
//! Phase 0 only declares the probe names so cards can reference them and the
//! validation can check them. The probe functions come in phase 1.

/// A probe known to the registry.
#[derive(Debug, Clone, Copy)]
pub struct ProbeInfo {
    pub name: &'static str,
    pub description: &'static str,
}

pub const REGISTRY: &[ProbeInfo] = &[
    ProbeInfo {
        name: "mem.summary",
        description: "RAM and swap: total, free, available, cache, swappiness",
    },
    ProbeInfo {
        name: "disk.tree",
        description: "disks, partitions, mount points and free space",
    },
    ProbeInfo {
        name: "proc.states",
        description: "number of processes in each state, zombies included",
    },
];

pub fn find(name: &str) -> Option<&'static ProbeInfo> {
    REGISTRY.iter().find(|p| p.name == name)
}
