//! `fs.mounts`: the filesystems holding your data, and the mount options
//! that change how they behave (journal mode, copy-on-write, compression,
//! access times).

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "fs.mounts";

/// Filesystems that store files on a device or a network; pseudo ones
/// (proc, tmpfs, cgroup2…) are left out.
const DATA_FS: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "bcachefs", "vfat", "exfat", "ntfs",
    "ntfs3", "fuseblk", "nfs", "nfs4", "cifs", "smb3", "virtiofs", "9p",
];

/// Options worth a word: these exactly, or starting with one of `PREFIXES`.
const EXACT: &[&str] = &[
    "ro",
    "noatime",
    "strictatime",
    "nodatacow",
    "nodatasum",
    "ssd",
    "sync",
    "nobarrier",
    "barrier=0",
];
const PREFIXES: &[&str] = &[
    "data=", "compress", "discard", "errors=", "subvol=", "commit=",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FsMounts {
    pub mounts: Vec<FsMount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FsMount {
    pub point: String,
    pub fstype: String,
    pub source: String,
    /// Telling options, as written: `noatime`, `compress=zstd:3`.
    pub options: Vec<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/self/mountinfo") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let mut mounts = Vec::new();
    for line in text.lines() {
        // `ID PARENT DEV ROOT POINT OPTIONS … - TYPE SOURCE SUPER_OPTIONS`
        let Some((before, after)) = line.split_once(" - ") else {
            continue;
        };
        let b: Vec<&str> = before.split_whitespace().collect();
        let a: Vec<&str> = after.split_whitespace().collect();
        if b.len() < 6 || a.len() < 2 || !DATA_FS.contains(&a[0]) {
            continue;
        }
        let point = super::disk::unescape(b[4]);
        let mut options: Vec<String> = Vec::new();
        for opt in b[5].split(',').chain(a.get(2).unwrap_or(&"").split(',')) {
            let keep = EXACT.contains(&opt) || PREFIXES.iter().any(|p| opt.starts_with(p));
            if keep && !options.iter().any(|o| o == opt) {
                options.push(opt.to_string());
            }
        }
        mounts.push(FsMount {
            point,
            fstype: a[0].to_string(),
            source: a[1].to_string(),
            options,
        });
    }
    ProbeResult::new(
        NAME,
        Status::Ok,
        Some(ProbeData::FsMounts(FsMounts { mounts })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn data_filesystems_and_their_options() {
        let Some(ProbeData::FsMounts(m)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        let points: Vec<&str> = m.mounts.iter().map(|m| m.point.as_str()).collect();
        // tmpfs, proc, devtmpfs and the overlay are left out.
        assert_eq!(
            points,
            [
                "/",
                "/home",
                "/var/lib/my data",
                "/boot/efi",
                "/media/usb",
                "/home"
            ]
        );
        assert_eq!(m.mounts[0].options, ["ssd", "subvol=/@"]);
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
