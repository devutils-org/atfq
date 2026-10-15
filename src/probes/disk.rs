//! `disk.tree`: disks, partitions, where they are mounted and how full they
//! are. Reads /sys/block, /proc/self/mountinfo and /proc/swaps, then asks
//! statvfs for the space of each mounted filesystem.

use serde::Serialize;

use super::{FsSpace, ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "disk.tree";
pub(crate) const TMP: &str = "fs.tmp";
pub(crate) const FSTAB: &str = "fs.fstab";
pub(crate) const UDEV: &str = "dev.udev";
pub(crate) const HARDWARE: &str = "disk.hardware";
pub(crate) const IO: &str = "disk.io";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskTree {
    pub devices: Vec<BlockDevice>,
    /// Loop and RAM devices, and empty devices, left out of `devices`.
    pub hidden: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockDevice {
    /// Kernel name: `sda`, `nvme0n1`, `dm-0`.
    pub name: String,
    /// Device-mapper name (`cryptroot`, `vg-root`), for `dm-*` devices.
    pub mapper_name: Option<String>,
    pub size: u64,
    pub model: Option<String>,
    /// `false` for SSDs and flash. Virtual disks often claim `true`.
    pub rotational: Option<bool>,
    pub removable: bool,
    pub swap: bool,
    /// Filesystems on the whole device (no partition table).
    pub mounts: Vec<Mount>,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Partition {
    pub name: String,
    pub number: u32,
    pub size: u64,
    pub swap: bool,
    pub mounts: Vec<Mount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mount {
    pub point: String,
    pub fstype: String,
    pub space: Option<FsSpace>,
}

/// One line of /proc/self/mountinfo.
struct MountEntry {
    dev: String,
    point: String,
    fstype: String,
    source: String,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let names = match ctx.list("/sys/block") {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let mut notes = Vec::new();
    let mounts = match ctx.read("/proc/self/mountinfo") {
        Ok(t) => parse_mountinfo(&t),
        Err(_) => {
            notes.push("/proc/self/mountinfo: can't read it, mount points unknown".to_string());
            Vec::new()
        }
    };
    let swaps: Vec<String> = ctx
        .read("/proc/swaps")
        .map(|t| {
            t.lines()
                .skip(1)
                .filter_map(|l| l.split_whitespace().next().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut tree = DiskTree {
        devices: Vec::new(),
        hidden: 0,
    };
    for name in names {
        let base = format!("/sys/block/{name}");
        let size = sectors(ctx, &base);
        if size == 0 || name.starts_with("loop") || name.starts_with("ram") {
            tree.hidden += 1;
            continue;
        }
        let mapper_name = ctx.read_trimmed(&format!("{base}/dm/name"));
        let mut aliases = vec![format!("/dev/{name}")];
        if let Some(m) = &mapper_name {
            aliases.push(format!("/dev/mapper/{m}"));
        }
        let mut partitions: Vec<Partition> = ctx
            .list(&base)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|child| {
                let dir = format!("{base}/{child}");
                let number = ctx
                    .read_trimmed(&format!("{dir}/partition"))?
                    .parse()
                    .ok()?;
                let dev_path = format!("/dev/{child}");
                Some(Partition {
                    number,
                    size: sectors(ctx, &dir),
                    swap: swaps.contains(&dev_path),
                    mounts: mounts_of(ctx, &mounts, &dir, &[dev_path]),
                    name: child,
                })
            })
            .collect();
        partitions.sort_by_key(|p| p.number);

        tree.devices.push(BlockDevice {
            size,
            model: ctx
                .read_trimmed(&format!("{base}/device/model"))
                .filter(|m| !m.is_empty()),
            rotational: ctx
                .read_trimmed(&format!("{base}/queue/rotational"))
                .map(|r| r == "1"),
            removable: ctx.read_trimmed(&format!("{base}/removable")).as_deref() == Some("1"),
            swap: aliases.iter().any(|a| swaps.contains(a)),
            mounts: mounts_of(ctx, &mounts, &base, &aliases),
            partitions,
            mapper_name,
            name,
        });
    }

    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(NAME, status, Some(ProbeData::Disk(tree)));
    r.notes = notes;
    r
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TmpDirs {
    pub dirs: Vec<TmpDir>,
}

/// Where a temporary directory lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TmpDir {
    pub path: String,
    /// The mount that holds it: itself when it is a separate mount.
    pub mount_point: String,
    pub fstype: String,
    pub space: Option<FsSpace>,
}

/// `fs.tmp`: is /tmp in RAM (tmpfs) or on disk, and how much room it has.
pub(crate) fn tmp(ctx: &ProbeCtx) -> ProbeResult {
    let mounts = match ctx.read("/proc/self/mountinfo") {
        Ok(t) => parse_mountinfo(&t),
        Err(e) => return ProbeResult::failed(TMP, e),
    };
    let dirs = ["/tmp", "/var/tmp"]
        .iter()
        .filter_map(|path| {
            // The deepest mount point that contains the path holds it.
            let m = mounts
                .iter()
                .filter(|m| contains(&m.point, path))
                .max_by_key(|m| m.point.len())?;
            Some(TmpDir {
                path: path.to_string(),
                mount_point: m.point.clone(),
                fstype: m.fstype.clone(),
                space: ctx.space(&m.point),
            })
        })
        .collect();
    ProbeResult::new(TMP, Status::Ok, Some(ProbeData::Tmp(TmpDirs { dirs })))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fstab {
    pub entries: Vec<FstabEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FstabEntry {
    /// `UUID=…`, `/dev/sda1`, `LABEL=…`, a server path…
    pub source: String,
    pub mount_point: String,
    pub fstype: String,
    pub options: String,
    /// Whether it is mounted now; `None` for swap.
    pub mounted: Option<bool>,
}

/// `fs.fstab`: what /etc/fstab asks for, and whether it is mounted now.
pub(crate) fn fstab(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/etc/fstab") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(FSTAB, e),
    };
    let mounts = ctx
        .read("/proc/self/mountinfo")
        .map(|t| parse_mountinfo(&t))
        .ok();
    let entries = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let (source, point, fstype) = (*f.first()?, *f.get(1)?, *f.get(2)?);
            let point = unescape(point);
            let mounted = (fstype != "swap")
                .then(|| mounts.as_ref().map(|m| m.iter().any(|e| e.point == point)));
            Some(FstabEntry {
                source: unescape(source),
                fstype: fstype.to_string(),
                options: f.get(3).unwrap_or(&"defaults").to_string(),
                mounted: mounted.flatten(),
                mount_point: point,
            })
        })
        .collect();
    ProbeResult::new(FSTAB, Status::Ok, Some(ProbeData::Fstab(Fstab { entries })))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UdevRules {
    /// Local rules, in /etc/udev/rules.d.
    pub local: Vec<String>,
    /// Rules installed by packages.
    pub packaged: u32,
}

/// `dev.udev`: local udev rules, next to the packaged ones.
pub(crate) fn udev(ctx: &ProbeCtx) -> ProbeResult {
    let rules = |dir: &str| -> Option<Vec<String>> {
        let list = ctx.list(dir).ok()?;
        Some(list.into_iter().filter(|f| f.ends_with(".rules")).collect())
    };
    let packaged = ["/usr/lib/udev/rules.d", "/lib/udev/rules.d"]
        .iter()
        .find_map(|d| rules(d));
    let local = rules("/etc/udev/rules.d");
    if packaged.is_none() && local.is_none() {
        let mut r = ProbeResult::new(UDEV, Status::Unavailable, None);
        r.notes
            .push("no udev rules folder: udev isn't used here".to_string());
        return r;
    }
    let data = UdevRules {
        local: local.unwrap_or_default(),
        packaged: packaged.map_or(0, |p| p.len() as u32),
    };
    ProbeResult::new(UDEV, Status::Ok, Some(ProbeData::Udev(data)))
}

fn contains(mount_point: &str, path: &str) -> bool {
    mount_point == "/"
        || path == mount_point
        || path
            .strip_prefix(mount_point)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Disks backed by real (or virtual) hardware: no loop, RAM or zram devices,
/// no device-mapper or RAID layers, nothing empty.
fn physical_disks(ctx: &ProbeCtx) -> Result<Vec<String>, super::ReadError> {
    Ok(ctx
        .list("/sys/block")?
        .into_iter()
        .filter(|n| {
            !["loop", "ram", "zram", "dm-", "md"]
                .iter()
                .any(|p| n.starts_with(p))
                && sectors(ctx, &format!("/sys/block/{n}")) > 0
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskHardware {
    pub disks: Vec<DiskHw>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskHw {
    pub name: String,
    pub model: Option<String>,
    pub size: u64,
    /// How it's attached: `NVMe`, `SATA/SAS`, `USB`, `virtio`, `SD/eMMC`.
    pub transport: String,
    /// `false` for flash. Virtual disks often claim `true`.
    pub rotational: Option<bool>,
    /// Active I/O scheduler and the others available.
    pub scheduler: Option<String>,
    pub schedulers: Vec<String>,
    /// Hardware queues (blk-mq): NVMe has one per CPU or so.
    pub queues: u32,
    /// `write back`: the drive has a volatile cache the kernel must flush.
    pub write_cache: Option<String>,
    /// The device accepts discard (TRIM).
    pub discard: bool,
    pub logical_block: Option<u64>,
    pub physical_block: Option<u64>,
}

pub(crate) fn hardware(ctx: &ProbeCtx) -> ProbeResult {
    let names = match physical_disks(ctx) {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(HARDWARE, e),
    };
    let disks = names
        .into_iter()
        .map(|name| {
            let base = format!("/sys/block/{name}");
            let q = |f: &str| ctx.read_trimmed(&format!("{base}/queue/{f}"));
            let num = |f: &str| q(f).and_then(|v| v.parse::<u64>().ok());
            // `none [mq-deadline] kyber`: the active one is in brackets.
            let line = q("scheduler").unwrap_or_default();
            let scheduler = line
                .split_whitespace()
                .find(|s| s.starts_with('['))
                .map(|s| s.trim_matches(['[', ']']).to_string());
            let schedulers = line
                .split_whitespace()
                .map(|s| s.trim_matches(['[', ']']).to_string())
                .collect();
            // /sys/block/X links to the device's place on the buses.
            let link = ctx.read_link(&base).unwrap_or_default();
            let transport = if name.starts_with("nvme") {
                "NVMe"
            } else if name.starts_with("mmcblk") {
                "SD/eMMC"
            } else if name.starts_with("vd") || link.contains("/virtio") {
                "virtio"
            } else if link.contains("/usb") {
                "USB"
            } else if name.starts_with("sd") {
                "SATA/SAS"
            } else {
                "other"
            };
            DiskHw {
                model: ctx
                    .read_trimmed(&format!("{base}/device/model"))
                    .filter(|m| !m.is_empty()),
                size: sectors(ctx, &base),
                transport: transport.to_string(),
                rotational: q("rotational").map(|r| r == "1"),
                scheduler,
                schedulers,
                queues: ctx
                    .list(&format!("{base}/mq"))
                    .map_or(0, |l| l.len() as u32),
                write_cache: q("write_cache"),
                discard: num("discard_max_bytes").is_some_and(|d| d > 0),
                logical_block: num("logical_block_size"),
                physical_block: num("physical_block_size"),
                name,
            }
        })
        .collect();
    ProbeResult::new(
        HARDWARE,
        Status::Ok,
        Some(ProbeData::DiskHardware(DiskHardware { disks })),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskIo {
    pub disks: Vec<DiskStats>,
    /// Seconds since boot, the period the counters cover.
    pub uptime_secs: Option<u64>,
    /// Bytes written by programs but not yet on disk, and being written now.
    pub dirty: Option<u64>,
    pub writeback: Option<u64>,
    /// `vm.dirty_expire_centisecs`: how old dirty data gets before it's
    /// written, in seconds.
    pub dirty_expire_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskStats {
    pub name: String,
    pub reads: u64,
    pub read_bytes: u64,
    /// Milliseconds spent on all reads, added up.
    pub read_ms: u64,
    pub writes: u64,
    pub write_bytes: u64,
    pub write_ms: u64,
    /// Milliseconds with at least one request in flight.
    pub busy_ms: u64,
    /// Cache flushes asked of the drive (fsync and friends), since 5.5.
    pub flushes: Option<u64>,
}

pub(crate) fn io(ctx: &ProbeCtx) -> ProbeResult {
    let text = match ctx.read("/proc/diskstats") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(IO, e),
    };
    let keep = physical_disks(ctx).ok();
    let disks = text
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split_whitespace().collect();
            let name = f.get(2)?.to_string();
            if !keep.as_ref().is_some_and(|k| k.contains(&name)) {
                return None;
            }
            let n = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok());
            // Sectors here are always 512 bytes.
            Some(DiskStats {
                reads: n(3)?,
                read_bytes: n(5)?.saturating_mul(512),
                read_ms: n(6)?,
                writes: n(7)?,
                write_bytes: n(9)?.saturating_mul(512),
                write_ms: n(10)?,
                busy_ms: n(12)?,
                flushes: n(18),
                name,
            })
        })
        .collect();
    let meminfo = ctx.read("/proc/meminfo").unwrap_or_default();
    let kb = |k: &str| {
        meminfo.lines().find_map(|l| {
            let rest = l.strip_prefix(k)?.strip_prefix(':')?;
            rest.split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
                .map(|v| v * 1024)
        })
    };
    let data = DiskIo {
        disks,
        uptime_secs: ctx
            .read_trimmed("/proc/uptime")
            .and_then(|u| u.split('.').next()?.parse().ok()),
        dirty: kb("Dirty"),
        writeback: kb("Writeback"),
        dirty_expire_secs: ctx
            .read_trimmed("/proc/sys/vm/dirty_expire_centisecs")
            .and_then(|c| c.parse::<u64>().ok())
            .map(|c| c / 100),
    };
    ProbeResult::new(IO, Status::Ok, Some(ProbeData::DiskIo(data)))
}

/// Size in bytes. sysfs counts in 512-byte sectors, whatever the hardware.
fn sectors(ctx: &ProbeCtx, dir: &str) -> u64 {
    ctx.read_trimmed(&format!("{dir}/size"))
        .and_then(|s| s.parse::<u64>().ok())
        .map_or(0, |n| n.saturating_mul(512))
}

/// Mounts of the block device in `dir`. A mount matches by device number,
/// or by its source path: btrfs subvolumes report an anonymous device number.
fn mounts_of(ctx: &ProbeCtx, all: &[MountEntry], dir: &str, sources: &[String]) -> Vec<Mount> {
    let dev = ctx.read_trimmed(&format!("{dir}/dev"));
    let mut found: Vec<Mount> = Vec::new();
    for m in all {
        let hit = dev.as_deref() == Some(m.dev.as_str()) || sources.contains(&m.source);
        if hit && !found.iter().any(|f| f.point == m.point) {
            found.push(Mount {
                point: m.point.clone(),
                fstype: m.fstype.clone(),
                space: ctx.space(&m.point),
            });
        }
    }
    // `/` first, then the shortest paths: the most telling ones.
    found.sort_by(|a, b| {
        (a.point != "/", a.point.len(), &a.point).cmp(&(b.point != "/", b.point.len(), &b.point))
    });
    found
}

/// `36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw`
fn parse_mountinfo(text: &str) -> Vec<MountEntry> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let sep = fields.iter().position(|&f| f == "-")?;
            Some(MountEntry {
                dev: fields.get(2)?.to_string(),
                point: unescape(fields.get(4)?),
                fstype: fields.get(sep + 1)?.to_string(),
                source: fields.get(sep + 2)?.to_string(),
            })
        })
        .collect()
}

/// mountinfo writes space, tab, newline and backslash as `\040`-style octal.
fn unescape(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let oct = bytes.get(i + 1..i + 4).unwrap_or_default();
        if bytes[i] == b'\\' && oct.len() == 3 && oct.iter().all(|b| (b'0'..=b'7').contains(b)) {
            let value = oct
                .iter()
                .fold(0u32, |acc, b| acc * 8 + u32::from(b - b'0'));
            if let Ok(b) = u8::try_from(value) {
                out.push(b);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    const GIB: u64 = 1 << 30;

    fn laptop() -> ProbeCtx {
        fixture("laptop")
            .with_space("/", space(400 * GIB, 120 * GIB, 0, 0))
            .with_space("/boot/efi", space(GIB / 2, GIB / 4, 0, 0))
            .with_space("/tmp", space(8 * GIB, 7 * GIB, 2_000_000, 1_998_000))
            .with_addr("lo", "127.0.0.1/8")
            .with_addr("lo", "::1/128")
            .with_addr("wlp0s20f3", "192.168.1.23/24")
            .with_addr("wlp0s20f3", "fe80::8ec6:81ff:fe2a:107f/64")
            .with_addr("docker0", "172.17.0.1/16")
    }

    fn space(total: u64, available: u64, inodes_total: u64, inodes_free: u64) -> FsSpace {
        FsSpace {
            total,
            available,
            inodes_total,
            inodes_free,
        }
    }

    fn tree(ctx: &ProbeCtx) -> DiskTree {
        match probe(ctx).data {
            Some(ProbeData::Disk(t)) => t,
            other => panic!("no data: {other:?}"),
        }
    }

    #[test]
    fn builds_the_tree() {
        let t = tree(&laptop());
        // loop0 and an empty nbd0 are hidden.
        assert_eq!(t.hidden, 2);
        let names: Vec<&str> = t.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["nvme0n1", "sda", "zram0"]);

        let nvme = &t.devices[0];
        assert_eq!(nvme.size, 1_000_215_216 * 512);
        assert_eq!(nvme.model.as_deref(), Some("Samsung SSD 980 PRO 500GB"));
        assert_eq!(nvme.rotational, Some(false));
        let parts: Vec<(&str, u32)> = nvme
            .partitions
            .iter()
            .map(|p| (p.name.as_str(), p.number))
            .collect();
        // Sorted by number, not by name.
        assert_eq!(
            parts,
            [("nvme0n1p1", 1), ("nvme0n1p2", 2), ("nvme0n1p10", 10)]
        );

        // Root is a btrfs subvolume: matched by source, `/` listed first.
        let root = &nvme.partitions[1];
        let points: Vec<&str> = root.mounts.iter().map(|m| m.point.as_str()).collect();
        assert_eq!(points, ["/", "/home", "/var/lib/my data"]);
        assert_eq!(root.mounts[0].fstype, "btrfs");
        assert_eq!(root.mounts[0].space.unwrap().available, 120 * GIB);

        let efi = &nvme.partitions[0];
        assert_eq!(efi.mounts[0].point, "/boot/efi");
        assert_eq!(efi.mounts[0].fstype, "vfat");

        let usb = &t.devices[1];
        assert!(usb.removable);
        assert_eq!(usb.rotational, Some(true));
        assert!(usb.partitions.is_empty());
        assert_eq!(usb.mounts[0].point, "/media/usb");
        // No space known for it in the fixture.
        assert_eq!(usb.mounts[0].space, None);

        assert!(t.devices[2].swap);
    }

    #[test]
    fn without_mountinfo_is_partial() {
        let r = probe(&fixture("old-kernel"));
        assert_eq!(r.status, Status::Partial);
        assert!(r.notes[0].contains("mountinfo"));
        let Some(ProbeData::Disk(t)) = r.data else {
            panic!()
        };
        assert_eq!(t.devices.len(), 1);
        assert!(t.devices[0].mounts.is_empty());
        assert_eq!(t.devices[0].model, None);
    }

    #[test]
    fn missing_sys_block_is_unavailable() {
        let r = probe(&fixture("empty"));
        assert_eq!(r.status, Status::Unavailable);
        assert_eq!(r.notes, ["/sys/block: not found"]);
    }

    #[test]
    fn finds_where_tmp_lives() {
        let Some(ProbeData::Tmp(t)) = tmp(&laptop()).data else {
            panic!()
        };
        let got: Vec<(&str, &str, &str)> = t
            .dirs
            .iter()
            .map(|d| (d.path.as_str(), d.mount_point.as_str(), d.fstype.as_str()))
            .collect();
        assert_eq!(got, [("/tmp", "/tmp", "tmpfs"), ("/var/tmp", "/", "btrfs")]);
        assert!(contains("/", "/var/tmp"));
        assert!(!contains("/tm", "/tmp"));
        assert_eq!(tmp(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn fstab_against_mounts() {
        let Some(ProbeData::Fstab(f)) = fstab(&laptop()).data else {
            panic!()
        };
        let got: Vec<(&str, &str, Option<bool>)> = f
            .entries
            .iter()
            .map(|e| (e.mount_point.as_str(), e.fstype.as_str(), e.mounted))
            .collect();
        assert_eq!(
            got,
            [
                ("/", "btrfs", Some(true)),
                ("/boot/efi", "vfat", Some(true)),
                ("none", "swap", None),
                ("/mnt/nas photos", "nfs", Some(false)),
            ]
        );
        assert_eq!(fstab(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn udev_rules() {
        let Some(ProbeData::Udev(u)) = udev(&laptop()).data else {
            panic!()
        };
        assert_eq!(u.local, ["70-keyboard.rules"]);
        assert_eq!(u.packaged, 2);
        assert_eq!(udev(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn disk_hardware() {
        let Some(ProbeData::DiskHardware(h)) = hardware(&fixture("laptop")).data else {
            panic!()
        };
        let names: Vec<&str> = h.disks.iter().map(|d| d.name.as_str()).collect();
        // No loop, zram or empty devices.
        assert_eq!(names, ["nvme0n1", "sda"]);
        let nvme = &h.disks[0];
        assert_eq!(nvme.transport, "NVMe");
        assert_eq!(nvme.scheduler.as_deref(), Some("none"));
        assert_eq!(nvme.schedulers, ["none", "mq-deadline", "kyber", "bfq"]);
        assert_eq!(nvme.queues, 8);
        assert_eq!(nvme.write_cache.as_deref(), Some("write back"));
        assert!(nvme.discard);
        // /sys/block/sda links into the USB tree.
        let usb = &h.disks[1];
        assert_eq!(usb.transport, "USB");
        assert_eq!(usb.scheduler.as_deref(), Some("mq-deadline"));
        assert!(!usb.discard);
        let Some(ProbeData::DiskHardware(h)) = hardware(&fixture("server")).data else {
            panic!()
        };
        assert_eq!(h.disks[0].transport, "SATA/SAS");
        assert_eq!(h.disks[0].physical_block, Some(4096));
        assert_eq!(hardware(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn disk_activity() {
        let Some(ProbeData::DiskIo(io)) = io(&fixture("laptop")).data else {
            panic!()
        };
        let names: Vec<&str> = io.disks.iter().map(|d| d.name.as_str()).collect();
        // Partitions, loop and zram are left out.
        assert_eq!(names, ["nvme0n1", "sda"]);
        let nvme = &io.disks[0];
        assert_eq!(nvme.reads, 2_843_117);
        assert_eq!(nvme.read_bytes, 198_765_432 * 512);
        assert_eq!(nvme.write_ms, 2_345_678);
        assert_eq!(nvme.busy_ms, 1_987_654);
        assert_eq!(nvme.flushes, Some(345_678));
        assert_eq!(io.disks[1].flushes, Some(0));
        assert_eq!(io.dirty, Some(1536 * 1024));
        assert_eq!(io.dirty_expire_secs, Some(30));
        assert_eq!(io.uptime_secs, Some(273_912));
        assert_eq!(super::io(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn unescapes_mount_points() {
        assert_eq!(unescape(r"/mnt/my\040disk"), "/mnt/my disk");
        assert_eq!(unescape(r"/a\134b"), r"/a\b");
        assert_eq!(unescape(r"/trailing\04"), r"/trailing\04");
        assert_eq!(unescape(r"/x\999"), r"/x\999");
    }
}
