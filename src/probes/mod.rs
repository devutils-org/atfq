//! Live probes: what this machine says.
//!
//! A probe is a pure function of a [`ProbeCtx`]. Every read of `/proc`, `/sys`
//! or `/etc` goes through the context, whose root is `/` in production and a
//! fixture directory in tests. Probes return structured data and a status;
//! they never render and never panic on a missing or unreadable file.

mod boot;
mod cpu;
mod disk;
mod kernel;
mod mem;
mod net;
mod proc;
mod sys;
mod systemd;
mod user;

use std::collections::BTreeMap;
use std::io;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

pub use boot::{BootInfo, Firmware};
pub use cpu::CpuSummary;
pub use disk::{BlockDevice, DiskTree, Mount, Partition, TmpDir, TmpDirs};
pub use kernel::{Modules, Sysctl, SysctlSetting, SysctlValue};
pub use mem::MemSummary;
pub use net::{DnsConfig, Interface, Listener, Listening, NetInterfaces};
pub use proc::{ProcInfo, ProcStates};
pub use sys::{FsSpace, IfAddr};
pub use systemd::{DropIn, FailedUnit, Overrides, SystemdStatus};
pub use user::Identity;

/// How far a probe got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    /// Some data is missing; `notes` says what.
    Partial,
    NeedsRoot,
    /// The files it reads are not there (old kernel, container…).
    Unavailable,
    /// Not Linux.
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ProbeData {
    Mem(MemSummary),
    Disk(DiskTree),
    Proc(ProcStates),
    Cpu(CpuSummary),
    Net(NetInterfaces),
    Listening(Listening),
    Dns(DnsConfig),
    Boot(BootInfo),
    User(Identity),
    Tmp(TmpDirs),
    Sysctl(Sysctl),
    Modules(Modules),
    Systemd(SystemdStatus),
    Overrides(Overrides),
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub name: &'static str,
    pub status: Status,
    pub data: Option<ProbeData>,
    /// Short notes for the user: what is missing and why.
    pub notes: Vec<String>,
}

impl ProbeResult {
    pub(crate) fn new(name: &'static str, status: Status, data: Option<ProbeData>) -> Self {
        ProbeResult {
            name,
            status,
            data,
            notes: Vec::new(),
        }
    }

    pub(crate) fn failed(name: &'static str, err: ReadError) -> Self {
        let (status, note) = match err {
            ReadError::NeedsRoot(p) => (Status::NeedsRoot, format!("{p}: permission denied")),
            ReadError::Missing(p) => (Status::Unavailable, format!("{p}: not found")),
            ReadError::Other(p, e) => (Status::Unavailable, format!("{p}: {e}")),
        };
        let mut r = ProbeResult::new(name, status, None);
        r.notes.push(note);
        r
    }
}

/// Where answers that need a system call (statvfs, getifaddrs) come from.
#[derive(Debug, Clone)]
enum Calls {
    Live,
    /// Fixtures: fixed answers, so tests don't depend on the host.
    Fixed {
        space: BTreeMap<String, FsSpace>,
        addrs: Vec<IfAddr>,
        /// Command line → output, for `run`.
        commands: BTreeMap<String, String>,
    },
}

/// Where probes read from.
#[derive(Debug, Clone)]
pub struct ProbeCtx {
    root: PathBuf,
    supported: bool,
    calls: Calls,
}

/// Why a read failed, with the path as the user knows it (`/proc/meminfo`).
#[derive(Debug, Clone)]
pub(crate) enum ReadError {
    Missing(String),
    NeedsRoot(String),
    Other(String, String),
}

impl ProbeCtx {
    /// The running machine.
    pub fn system() -> ProbeCtx {
        ProbeCtx {
            root: PathBuf::from("/"),
            supported: cfg!(target_os = "linux"),
            calls: Calls::Live,
        }
    }

    /// A fake tree, for tests. Filesystem space and IP addresses come from
    /// [`Self::with_space`] and [`Self::with_addr`].
    pub fn fixture(root: impl Into<PathBuf>) -> ProbeCtx {
        ProbeCtx {
            root: root.into(),
            supported: true,
            calls: Calls::Fixed {
                space: BTreeMap::new(),
                addrs: Vec::new(),
                commands: BTreeMap::new(),
            },
        }
    }

    /// Sets the space a fixture reports for `mount_point`.
    pub fn with_space(mut self, mount_point: &str, fs: FsSpace) -> ProbeCtx {
        if let Calls::Fixed { space, .. } = &mut self.calls {
            space.insert(mount_point.to_string(), fs);
        }
        self
    }

    /// Adds an address a fixture reports, such as `("eth0", "192.168.1.20/24")`.
    pub fn with_addr(mut self, iface: &str, cidr: &str) -> ProbeCtx {
        if let Calls::Fixed { addrs, .. } = &mut self.calls {
            let (ip, prefix) = cidr.split_once('/').expect("address/prefix");
            addrs.push(IfAddr {
                iface: iface.to_string(),
                addr: ip.parse().expect("IP address"),
                prefix: prefix.parse().expect("prefix length"),
            });
        }
        self
    }

    /// Sets what a fixture's `run` returns for a command line.
    pub fn with_command(mut self, cmd: &[&str], output: &str) -> ProbeCtx {
        if let Calls::Fixed { commands, .. } = &mut self.calls {
            commands.insert(cmd.join(" "), output.to_string());
        }
        self
    }

    /// Runs an allowlisted program with fixed arguments (see [`sys::run`])
    /// and returns what it printed, whatever its exit code.
    pub(crate) fn run(&self, cmd: &[&str]) -> Result<String, ReadError> {
        match &self.calls {
            Calls::Fixed { commands, .. } => commands
                .get(&cmd.join(" "))
                .cloned()
                .ok_or_else(|| ReadError::Missing(cmd[0].to_string())),
            Calls::Live => sys::run(cmd),
        }
    }

    /// `abs` (an absolute system path) inside the root.
    pub(crate) fn path(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    pub(crate) fn read(&self, abs: &str) -> Result<String, ReadError> {
        std::fs::read_to_string(self.path(abs)).map_err(|e| read_error(abs, e))
    }

    pub(crate) fn read_bytes(&self, abs: &str) -> Result<Vec<u8>, ReadError> {
        std::fs::read(self.path(abs)).map_err(|e| read_error(abs, e))
    }

    /// Where a symbolic link points, as written in the link.
    pub(crate) fn read_link(&self, abs: &str) -> Option<String> {
        std::fs::read_link(self.path(abs))
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }

    pub(crate) fn exists(&self, abs: &str) -> bool {
        self.path(abs).exists()
    }

    /// Reads a small file and trims it; `None` on any error.
    pub(crate) fn read_trimmed(&self, abs: &str) -> Option<String> {
        self.read(abs).ok().map(|s| s.trim().to_string())
    }

    /// Names of the entries of a directory, sorted.
    pub(crate) fn list(&self, abs: &str) -> Result<Vec<String>, ReadError> {
        let dir = std::fs::read_dir(self.path(abs)).map_err(|e| read_error(abs, e))?;
        let mut names: Vec<String> = dir
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    /// Space of the filesystem mounted at `mount_point`.
    pub(crate) fn space(&self, mount_point: &str) -> Option<FsSpace> {
        match &self.calls {
            Calls::Fixed { space, .. } => space.get(mount_point).copied(),
            Calls::Live => sys::statvfs(&self.path(mount_point)),
        }
    }

    /// IP addresses of every interface; `None` if they can't be listed.
    pub(crate) fn addrs(&self) -> Option<Vec<IfAddr>> {
        match &self.calls {
            Calls::Fixed { addrs, .. } => Some(addrs.clone()),
            Calls::Live => sys::getifaddrs(),
        }
    }
}

fn read_error(abs: &str, e: io::Error) -> ReadError {
    match e.kind() {
        io::ErrorKind::NotFound => ReadError::Missing(abs.to_string()),
        io::ErrorKind::PermissionDenied => ReadError::NeedsRoot(abs.to_string()),
        _ => ReadError::Other(abs.to_string(), e.to_string()),
    }
}

/// A probe known to the registry.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProbeInfo {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(skip)]
    run: fn(&ProbeCtx) -> ProbeResult,
}

pub const REGISTRY: &[ProbeInfo] = &[
    ProbeInfo {
        name: mem::NAME,
        description: "RAM and swap: total, free, available, cache, swappiness",
        run: mem::probe,
    },
    ProbeInfo {
        name: disk::NAME,
        description: "disks, partitions, mount points and free space",
        run: disk::probe,
    },
    ProbeInfo {
        name: proc::NAME,
        description: "number of processes in each state, zombies included",
        run: proc::probe,
    },
    ProbeInfo {
        name: cpu::NAME,
        description: "CPU model and count, load average, frequency governor",
        run: cpu::probe,
    },
    ProbeInfo {
        name: net::INTERFACES,
        description: "network interfaces, their addresses, and the default route",
        run: net::interfaces,
    },
    ProbeInfo {
        name: net::LISTENING,
        description: "TCP and UDP ports open for connections, and who owns them",
        run: net::listening,
    },
    ProbeInfo {
        name: net::DNS,
        description: "DNS servers, search domains and name lookup order",
        run: net::dns,
    },
    ProbeInfo {
        name: boot::NAME,
        description: "firmware, Secure Boot, kernel command line, init, uptime",
        run: boot::probe,
    },
    ProbeInfo {
        name: user::NAME,
        description: "current user, groups and umask",
        run: user::probe,
    },
    ProbeInfo {
        name: disk::TMP,
        description: "where /tmp and /var/tmp live: RAM (tmpfs) or disk",
        run: disk::tmp,
    },
    ProbeInfo {
        name: kernel::SYSCTL,
        description: "telling kernel settings, and those set in /etc/sysctl.d",
        run: kernel::sysctl,
    },
    ProbeInfo {
        name: kernel::MODULES,
        description: "loaded kernel modules and taint flags",
        run: kernel::modules,
    },
    ProbeInfo {
        name: systemd::STATUS,
        description: "systemd's overall state and failed units (runs systemctl)",
        run: systemd::status,
    },
    ProbeInfo {
        name: systemd::OVERRIDES,
        description: "unit files and drop-ins changed in /etc/systemd/system",
        run: systemd::overrides,
    },
];

pub fn find(name: &str) -> Option<&'static ProbeInfo> {
    REGISTRY.iter().find(|p| p.name == name)
}

impl ProbeInfo {
    pub fn run(&self, ctx: &ProbeCtx) -> ProbeResult {
        if !ctx.supported {
            let mut r = ProbeResult::new(self.name, Status::Unsupported, None);
            r.notes.push("only available on Linux".to_string());
            return r;
        }
        (self.run)(ctx)
    }
}

/// Runs the named probes, in order. Unknown names are skipped (the content
/// validation guarantees there are none).
pub fn run_all(names: &[String], ctx: &ProbeCtx) -> Vec<ProbeResult> {
    names
        .iter()
        .filter_map(|n| find(n))
        .map(|p| p.run(ctx))
        .collect()
}

/// A fixture tree from `tests/fixtures/`.
#[cfg(test)]
pub(crate) fn fixture(name: &str) -> ProbeCtx {
    ProbeCtx::fixture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_os() {
        let mut ctx = fixture("laptop");
        ctx.supported = false;
        for p in REGISTRY {
            let r = p.run(&ctx);
            assert_eq!(r.status, Status::Unsupported);
            assert!(r.data.is_none());
        }
    }

    #[test]
    fn unreadable_file_needs_root() {
        let r = ProbeResult::failed("x", ReadError::NeedsRoot("/proc/kcore".into()));
        assert_eq!(r.status, Status::NeedsRoot);
        assert_eq!(r.notes, ["/proc/kcore: permission denied"]);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_files_do_not_panic() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("atfq-unreadable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("proc/1")).unwrap();
        fs::create_dir_all(dir.join("sys/block")).unwrap();
        fs::write(dir.join("proc/meminfo"), "MemTotal: 1 kB\n").unwrap();
        fs::write(dir.join("proc/1/stat"), "1 (init) S 0").unwrap();
        let locked = ["proc/meminfo", "proc/1/stat", "sys/block"];
        let mode = |p: &str, m: u32| {
            fs::set_permissions(dir.join(p), fs::Permissions::from_mode(m)).unwrap()
        };
        for p in locked {
            mode(p, 0o000);
        }
        // root reads everything anyway: then only "no panic" is checked.
        let as_root = fs::read_to_string(dir.join("proc/meminfo")).is_ok();
        let ctx = ProbeCtx::fixture(&dir);
        let statuses: Vec<Status> = REGISTRY.iter().map(|p| p.run(&ctx).status).collect();
        for p in locked {
            mode(p, 0o755);
        }
        fs::remove_dir_all(&dir).unwrap();
        if !as_root {
            // mem.summary, disk.tree, proc.states: the others find nothing.
            assert_eq!(
                statuses[..3],
                [Status::NeedsRoot, Status::NeedsRoot, Status::Unavailable]
            );
        }
    }

    #[test]
    fn run_all_keeps_order() {
        let names = vec!["proc.states".to_string(), "mem.summary".to_string()];
        let rs = run_all(&names, &fixture("laptop"));
        let got: Vec<&str> = rs.iter().map(|r| r.name).collect();
        assert_eq!(got, ["proc.states", "mem.summary"]);
    }
}
