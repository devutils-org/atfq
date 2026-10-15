//! `virt.detect`: is this a container, a virtual machine, or both? Which
//! namespaces are separate from the host's, and can this machine run
//! containers and VMs itself?

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "virt.detect";

/// Namespaces whose first (host) instance always has the same inode number,
/// set in the kernel (`PROC_*_INIT_INO`). Mount and network namespaces get
/// a number at boot, so they can't be compared this way.
const HOST_NS: &[(&str, u64)] = &[
    ("pid", 4026531836),
    ("user", 4026531837),
    ("uts", 4026531838),
    ("ipc", 4026531839),
    ("cgroup", 4026531835),
    ("time", 4026531834),
];

/// DMI vendor or product → hypervisor or cloud.
const VM_VENDORS: &[(&str, &str)] = &[
    ("QEMU", "QEMU/KVM"),
    ("KVM", "KVM"),
    ("innotek GmbH", "VirtualBox"),
    ("VMware", "VMware"),
    ("Xen", "Xen"),
    ("Amazon EC2", "Amazon EC2"),
    ("Google", "Google Compute Engine"),
    ("DigitalOcean", "DigitalOcean"),
    ("Hetzner", "Hetzner Cloud"),
    ("OpenStack", "OpenStack"),
    ("Parallels", "Parallels"),
    ("Apple Virtualization", "Apple Virtualization"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VirtInfo {
    /// Container type: `docker`, `podman`, `lxc`, `systemd-nspawn`…
    pub container: Option<String>,
    /// What gave it away: a file or a namespace.
    pub container_hint: Option<String>,
    /// Hypervisor or cloud, when this kernel runs in a virtual machine.
    pub vm: Option<String>,
    pub vm_hint: Option<String>,
    /// Namespaces checked, and whether this process has its own (not the
    /// host's).
    pub namespaces: Vec<(String, bool)>,
    /// /dev/kvm exists: this kernel can run hardware-accelerated VMs.
    pub kvm: bool,
    /// Overlay mounts: container layers, usually.
    pub overlays: u32,
    /// Container engines whose socket is present.
    pub engines: Vec<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let namespaces: Vec<(String, bool)> = HOST_NS
        .iter()
        .filter_map(|(name, host)| {
            // `pid:[4026531836]`
            let link = ctx.read_link(&format!("/proc/self/ns/{name}"))?;
            let ino: u64 = link.split_once('[')?.1.trim_end_matches(']').parse().ok()?;
            Some((name.to_string(), ino != *host))
        })
        .collect();
    if namespaces.is_empty() && !ctx.exists("/proc/self") {
        return ProbeResult::failed(NAME, super::ReadError::Missing("/proc/self".into()));
    }
    let (container, container_hint) = container(ctx, &namespaces);
    let (vm, vm_hint) = vm(ctx);
    let overlays = ctx
        .read("/proc/self/mountinfo")
        .map(|m| {
            m.lines()
                .filter(|l| {
                    l.split_once(" - ")
                        .is_some_and(|(_, after)| after.starts_with("overlay "))
                })
                .count() as u32
        })
        .unwrap_or(0);
    let engines = [
        ("/run/docker.sock", "Docker"),
        ("/run/podman/podman.sock", "Podman"),
        ("/run/containerd/containerd.sock", "containerd"),
        ("/run/crio/crio.sock", "CRI-O"),
        ("/var/lib/lxd/unix.socket", "LXD"),
        ("/var/snap/lxd/common/lxd/unix.socket", "LXD"),
        ("/run/incus/unix.socket", "Incus"),
    ]
    .iter()
    .filter(|(path, _)| ctx.exists(path))
    .map(|(_, name)| name.to_string())
    .fold(Vec::new(), |mut v: Vec<String>, n| {
        if !v.contains(&n) {
            v.push(n);
        }
        v
    });
    let data = VirtInfo {
        container,
        container_hint,
        vm,
        vm_hint,
        namespaces,
        kvm: ctx.exists("/dev/kvm"),
        overlays,
        engines,
    };
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Virt(data)))
}

fn container(ctx: &ProbeCtx, ns: &[(String, bool)]) -> (Option<String>, Option<String>) {
    let found = |kind: &str, hint: &str| (Some(kind.to_string()), Some(hint.to_string()));
    // systemd writes what it detected at boot.
    if let Some(kind) = ctx
        .read_trimmed("/run/systemd/container")
        .filter(|k| !k.is_empty())
    {
        return found(&kind, "/run/systemd/container");
    }
    if ctx.exists("/.dockerenv") {
        return found("docker", "/.dockerenv");
    }
    if ctx.exists("/run/.containerenv") {
        return found("podman", "/run/.containerenv");
    }
    let cgroup = ctx.read("/proc/1/cgroup").unwrap_or_default();
    for (needle, kind) in [
        ("/docker", "docker"),
        ("/kubepods", "kubernetes"),
        ("/lxc", "lxc"),
        ("libpod", "podman"),
    ] {
        if cgroup.contains(needle) {
            return found(kind, "/proc/1/cgroup");
        }
    }
    if ns.iter().any(|(n, own)| n == "pid" && *own) {
        return found("container", "a PID namespace of its own");
    }
    (None, None)
}

fn vm(ctx: &ProbeCtx) -> (Option<String>, Option<String>) {
    let found = |kind: &str, hint: &str| (Some(kind.to_string()), Some(hint.to_string()));
    let version = ctx.read("/proc/version").unwrap_or_default();
    if version.contains("microsoft") || version.contains("Microsoft") {
        return found("WSL 2 (Hyper-V)", "/proc/version");
    }
    if let Some(t) = ctx.read_trimmed("/sys/hypervisor/type") {
        return found(&t, "/sys/hypervisor/type");
    }
    for file in ["sys_vendor", "product_name", "bios_vendor"] {
        let path = format!("/sys/class/dmi/id/{file}");
        if let Some(v) = ctx.read_trimmed(&path) {
            if let Some((_, name)) = VM_VENDORS.iter().find(|(k, _)| v.contains(k)) {
                return found(name, &path);
            }
            if file == "product_name" && v == "Virtual Machine" {
                return found("Hyper-V", &path);
            }
        }
    }
    // ARM machines describe themselves in the device tree: QEMU's generic
    // board is `linux,dummy-virt`.
    if let Ok(dt) = ctx.read("/sys/firmware/devicetree/base/compatible") {
        let names: Vec<&str> = dt.split('\0').filter(|s| !s.is_empty()).collect();
        if let Some(n) = names
            .iter()
            .find(|n| n.ends_with(",virt") || n.ends_with("-virt"))
        {
            return found(n, "the device tree");
        }
    }
    // x86 CPUs tell a guest it runs under a hypervisor.
    let cpuinfo = ctx.read("/proc/cpuinfo").unwrap_or_default();
    let flagged = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .is_some_and(|l| l.split_whitespace().any(|f| f == "hypervisor"));
    if flagged {
        return found("unknown hypervisor", "the hypervisor CPU flag");
    }
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn info(name: &str) -> VirtInfo {
        let Some(ProbeData::Virt(v)) = probe(&fixture(name)).data else {
            panic!()
        };
        v
    }

    #[test]
    fn bare_metal_laptop_with_docker() {
        let v = info("laptop");
        assert_eq!((v.container, v.vm), (None, None));
        assert!(v.namespaces.iter().all(|(_, own)| !own));
        assert_eq!(v.namespaces.len(), 6);
        assert!(v.kvm);
        assert_eq!(v.overlays, 1);
        assert_eq!(v.engines, ["Docker"]);
    }

    #[test]
    fn docker_container_on_a_cloud_vm() {
        let v = info("container");
        assert_eq!(v.container.as_deref(), Some("docker"));
        assert_eq!(v.vm.as_deref(), Some("Amazon EC2"));
        let own: Vec<&str> = v
            .namespaces
            .iter()
            .filter(|(_, own)| *own)
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(own, ["pid", "uts", "ipc", "cgroup"]);
        assert!(!v.kvm);
    }

    #[test]
    fn empty_tree() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
