---
id: fsint.choose
kind: concept
title: "ext4, XFS, btrfs or ZFS?"
aliases: [ext4, xfs, which filesystem, choose filesystem, f2fs, bcachefs, exfat, fat32, vfat, ntfs, filesystem comparison]
hubs: [disk]
probes: [fs.mounts]
see_also: [fsint.cow, fsint.journal]
commands: [lsblk, findmnt]
paths: [/etc/fstab]
---

## TL;DR

- ext4: the safe default (Debian, Ubuntu). Mature, fast, repairable.
- XFS: big files and parallel workloads (RHEL's default). Can grow but
  never shrink.
- btrfs: snapshots, compression, checksums (Fedora's and openSUSE's
  default). ZFS: the same ideas, for storage servers with many disks.
- For a USB stick shared with Windows and macOS: exFAT.

```
lsblk -f                 # filesystem of each partition
findmnt -t ext4,xfs,btrfs
```

## Pitfalls

- FAT32 can't hold a file over 4 GiB: big copies fail with an odd
  error. exFAT has no such limit.
- Linux permissions don't exist on FAT and exFAT: every file gets the
  same owner and rights, set at mount time.
- Btrfs RAID 5 and 6 still carry warnings: use RAID 1 or 10 profiles, or
  ZFS, for parity RAID.
- Changing filesystem means reformatting: back up first.

## Deep dive

### Does the choice matter for speed?

For most desktops and servers, much less than the disk itself. The
differences show at the edges: XFS with many threads writing large
files, ext4 with lots of small metadata changes, btrfs slower on
rewrite-heavy databases unless copy-on-write is off for them. Features
and reliability are better reasons to choose.

### What about f2fs and bcachefs?

f2fs is designed for flash and is common on Android phones. bcachefs
aims to combine btrfs's features with ext4's reliability; it left the
mainline kernel in 2025 and is now maintained as a separate module.
