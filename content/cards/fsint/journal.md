---
id: fsint.journal
kind: concept
title: "Journaling: how a filesystem survives a crash"
aliases: [journaling, journaled filesystem, journal mode, data=ordered, fsck, e2fsck, filesystem corruption, read-only file system, remount-ro, xfs_repair]
hubs: [disk]
probes: [fs.mounts]
see_also: [fsint.layout, hw-storage.durability, kernel.dmesg]
commands: [fsck, dmesg, mount]
paths: [/etc/fstab]
---

## TL;DR

- Creating a file touches several places on disk: the inode, the folder,
  the free-space map. A crash in between would leave them disagreeing.
- A journal writes the whole change to a log area first, then applies
  it. After a crash, the filesystem replays or drops the logged changes:
  consistent again in seconds, without a full check.
- ext4 and XFS journal metadata; your file's content still needs fsync
  ({{hw-storage.durability}}).

```
findmnt -no OPTIONS /        # mount options of the root filesystem
sudo dmesg | grep -iE 'ext4|xfs|btrfs'   # errors and journal replays
```

## Pitfalls

- "Read-only file system" out of the blue: the kernel found an error and
  remounted it read-only (`errors=remount-ro`). Look at dmesg, then plan a
  check: the disk may be failing.
- Never run `fsck` on a mounted filesystem: it can destroy it. Check from
  a rescue system, or let it run at boot.
- `fsck -n` only reports problems; without it, fsck changes the disk.

## Deep dive

### What does ext4's default mode, data=ordered, promise?

Metadata goes through the journal; file data isn't journaled, but it's
written before the metadata that points to it is committed. So after a
crash, a file may hold its old content or its new content, never blocks
of someone else's deleted data. `data=journal` also journals the data:
safer, and much slower.

### Why does a journal make the check fast?

Without one, the only way to know the filesystem is consistent after a
crash is to read every inode and folder: minutes to hours on a large
disk. With a journal, only the last few seconds of changes can be
unfinished, and they're all in one small area.
