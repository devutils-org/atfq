---
id: fsint.cow
kind: concept
title: "Copy-on-write filesystems: snapshots and checksums"
aliases: [cow filesystem, btrfs, zfs, snapshot, snapshots, subvolume, subvolumes, bit rot, scrub, btrfs scrub, nodatacow, reflink, compression, zstd]
hubs: [disk]
probes: [fs.mounts]
see_also: [fsint.journal, fsint.choose]
commands: [btrfs, zfs, cp]
paths: []
---

## TL;DR

- Btrfs and ZFS never overwrite data in place: changes go to new blocks,
  then one pointer switches. The filesystem is always consistent.
- That makes snapshots almost free: a snapshot keeps the old pointers,
  and both share every unchanged block.
- They checksum data as well as metadata, so silent corruption (bit rot)
  is detected, and repaired if there's a second copy.

```
sudo btrfs subvolume snapshot / /.snapshots/before-upgrade
sudo btrfs scrub start /         # read everything, verify checksums
cp --reflink=always big.img copy.img   # instant copy, shares blocks
```

## Pitfalls

- A snapshot on the same disk isn't a backup: if the disk dies, both go.
- Databases and VM images, rewritten in place constantly, fragment badly
  under copy-on-write. `chattr +C` on their empty folder turns it off,
  and with it checksums and compression for those files.
- Old snapshots keep deleted data alive: free space comes back only when
  they're deleted.
- ZFS isn't in the Linux kernel (license): it comes as a separate module,
  which must be rebuilt for each new kernel.

## Deep dive

### How can a pointer switch be atomic?

Data and metadata form a tree. A change writes new copies of the changed
blocks, then new copies of their parents, up to the root. Until the root
is replaced, the old tree is complete and valid; replacing the root is
one small write. A crash at any point leaves either the old tree or the
new one.

### Why is compression often a free win?

With `compress=zstd` (btrfs) or `compression=zstd` (ZFS), text, logs and
code shrink to a half or a third. The disk then reads fewer blocks, and
fast CPUs decompress faster than most disks read, so it often speeds
things up as well as saving space.
