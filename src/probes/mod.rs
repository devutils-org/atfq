//! Live probes: what this machine says.
//!
//! A probe is a pure function of a [`ProbeCtx`]. Every read of `/proc`, `/sys`
//! or `/etc` goes through the context, whose root is `/` in production and a
//! fixture directory in tests. Probes return structured data and a status;
//! they never render and never panic on a missing or unreadable file.

mod disk;
mod mem;
mod proc;

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub use disk::{BlockDevice, DiskTree, Mount, Partition};
pub use mem::MemSummary;
pub use proc::{ProcInfo, ProcStates};

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

/// Space on a mounted filesystem, as `df` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FsSpace {
    pub total: u64,
    /// Free for normal users (excludes the blocks reserved for root).
    pub available: u64,
}

#[derive(Debug, Clone)]
enum SpaceSource {
    Statvfs,
    Fixed(BTreeMap<String, FsSpace>),
}

/// Where probes read from.
#[derive(Debug, Clone)]
pub struct ProbeCtx {
    root: PathBuf,
    supported: bool,
    space: SpaceSource,
}

/// Why a read failed, with the path as the user knows it (`/proc/meminfo`).
#[derive(Debug)]
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
            space: SpaceSource::Statvfs,
        }
    }

    /// A fake tree, for tests. Filesystem space comes from [`Self::with_space`].
    pub fn fixture(root: impl Into<PathBuf>) -> ProbeCtx {
        ProbeCtx {
            root: root.into(),
            supported: true,
            space: SpaceSource::Fixed(BTreeMap::new()),
        }
    }

    /// Sets the space a fixture reports for `mount_point`.
    pub fn with_space(mut self, mount_point: &str, space: FsSpace) -> ProbeCtx {
        if let SpaceSource::Fixed(map) = &mut self.space {
            map.insert(mount_point.to_string(), space);
        }
        self
    }

    /// `abs` (an absolute system path) inside the root.
    pub(crate) fn path(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    pub(crate) fn read(&self, abs: &str) -> Result<String, ReadError> {
        std::fs::read_to_string(self.path(abs)).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ReadError::Missing(abs.to_string()),
            io::ErrorKind::PermissionDenied => ReadError::NeedsRoot(abs.to_string()),
            _ => ReadError::Other(abs.to_string(), e.to_string()),
        })
    }

    /// Reads a small file and trims it; `None` on any error.
    pub(crate) fn read_trimmed(&self, abs: &str) -> Option<String> {
        self.read(abs).ok().map(|s| s.trim().to_string())
    }

    /// Names of the entries of a directory, sorted.
    pub(crate) fn list(&self, abs: &str) -> Result<Vec<String>, ReadError> {
        let dir = std::fs::read_dir(self.path(abs)).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ReadError::Missing(abs.to_string()),
            io::ErrorKind::PermissionDenied => ReadError::NeedsRoot(abs.to_string()),
            _ => ReadError::Other(abs.to_string(), e.to_string()),
        })?;
        let mut names: Vec<String> = dir
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    /// Space of the filesystem mounted at `mount_point`.
    pub(crate) fn space(&self, mount_point: &str) -> Option<FsSpace> {
        match &self.space {
            SpaceSource::Fixed(map) => map.get(mount_point).copied(),
            SpaceSource::Statvfs => statvfs(&self.path(mount_point)),
        }
    }
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // statvfs field types differ between platforms
fn statvfs(path: &Path) -> Option<FsSpace> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `c` is a valid NUL-terminated path and `st` is a valid out
    // pointer; statvfs only writes to it, and we read it only on success.
    let rc = unsafe { libc::statvfs(c.as_ptr(), st.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs returned 0, so it filled the struct.
    let st = unsafe { st.assume_init() };
    let frsize = st.f_frsize as u64;
    Some(FsSpace {
        total: st.f_blocks as u64 * frsize,
        available: st.f_bavail as u64 * frsize,
    })
}

#[cfg(not(unix))]
fn statvfs(_: &Path) -> Option<FsSpace> {
    None
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
            assert_eq!(
                statuses,
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
