//! Answers that need more than reading a file: filesystem space (statvfs),
//! IP addresses (getifaddrs), and the few external programs allowed (only
//! `systemctl`, whose state lives in systemd's memory). Tests replace all
//! of them with fixed values through `ProbeCtx`.

use super::ReadError;

use std::io::Read;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

/// Space on a mounted filesystem, as `df` and `df -i` report it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FsSpace {
    pub total: u64,
    /// Free for normal users (excludes the blocks reserved for root).
    pub available: u64,
    /// 0 when the filesystem has no fixed number of inodes (btrfs, for one).
    pub inodes_total: u64,
    pub inodes_free: u64,
}

/// One address of a network interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IfAddr {
    pub iface: String,
    pub addr: IpAddr,
    /// Prefix length: 24 in 192.168.1.20/24.
    pub prefix: u8,
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // statvfs field types differ between platforms
pub(crate) fn statvfs(path: &Path) -> Option<FsSpace> {
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
        inodes_total: st.f_files as u64,
        inodes_free: st.f_ffree as u64,
    })
}

#[cfg(not(unix))]
pub(crate) fn statvfs(_: &Path) -> Option<FsSpace> {
    None
}

#[cfg(unix)]
pub(crate) fn getifaddrs() -> Option<Vec<IfAddr>> {
    use std::ffi::CStr;
    use std::net::{Ipv4Addr, Ipv6Addr};

    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `head` with a list we free below.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut cur = head;
    while !cur.is_null() {
        // SAFETY: `cur` is a node of the list getifaddrs returned, not yet freed.
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_addr.is_null() || ifa.ifa_name.is_null() {
            continue;
        }
        // SAFETY: ifa_name is a NUL-terminated string owned by the list.
        let iface = unsafe { CStr::from_ptr(ifa.ifa_name) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: ifa_addr is non-null; its family says which sockaddr it is.
        let family = i32::from(unsafe { (*ifa.ifa_addr).sa_family });
        let (addr, prefix) = match family {
            libc::AF_INET => {
                // SAFETY: AF_INET means a sockaddr_in, and so is the netmask.
                let sin = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in) };
                let addr = IpAddr::V4(Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)));
                let prefix = if ifa.ifa_netmask.is_null() {
                    32
                } else {
                    let m = unsafe { &*(ifa.ifa_netmask as *const libc::sockaddr_in) };
                    m.sin_addr.s_addr.count_ones() as u8
                };
                (addr, prefix)
            }
            libc::AF_INET6 => {
                // SAFETY: AF_INET6 means a sockaddr_in6, and so is the netmask.
                let sin6 = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in6) };
                let addr = IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.s6_addr));
                let prefix = if ifa.ifa_netmask.is_null() {
                    128
                } else {
                    let m = unsafe { &*(ifa.ifa_netmask as *const libc::sockaddr_in6) };
                    m.sin6_addr
                        .s6_addr
                        .iter()
                        .map(|b| b.count_ones())
                        .sum::<u32>() as u8
                };
                (addr, prefix)
            }
            _ => continue,
        };
        out.push(IfAddr {
            iface,
            addr,
            prefix,
        });
    }
    // SAFETY: `head` came from getifaddrs and is freed once.
    unsafe { libc::freeifaddrs(head) };
    Some(out)
}

#[cfg(not(unix))]
pub(crate) fn getifaddrs() -> Option<Vec<IfAddr>> {
    None
}

/// The only programs a probe may run.
const ALLOWED: &[&str] = &["systemctl"];

/// A program that takes longer is killed and reported unavailable.
const TIMEOUT: Duration = Duration::from_millis(500);

/// Runs `cmd` (an allowlisted program and fixed arguments, never user
/// input), found through PATH. Returns its standard output, whatever the
/// exit code: `systemctl is-system-running` exits with 1 when "degraded".
pub(crate) fn run(cmd: &[&str]) -> Result<String, ReadError> {
    let name = cmd[0];
    if !ALLOWED.contains(&name) {
        return Err(ReadError::Other(
            name.to_string(),
            "not allowed".to_string(),
        ));
    }
    let path = find_in_path(name).ok_or_else(|| ReadError::Missing(name.to_string()))?;
    let mut child = Command::new(path)
        .args(&cmd[1..])
        .env("SYSTEMD_COLORS", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ReadError::Other(name.to_string(), e.to_string()))?;
    // Read in a thread: a full pipe would otherwise block the child.
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        out
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < TIMEOUT => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ReadError::Other(name.to_string(), "timed out".to_string()));
            }
        }
    }
    Ok(reader.join().unwrap_or_default())
}

#[cfg(not(unix))]
fn find_in_path(_: &str) -> Option<PathBuf> {
    None
}

#[cfg(unix)]
fn find_in_path(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .find(|p| {
            p.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}
