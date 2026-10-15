---
id: hw-storage.durability
kind: concept
title: "fsync: when is data really on disk?"
aliases: [fsync, fdatasync, durability, power loss, power cut, write cache, data loss, atomic write, atomic rename, crash safety, barrier, flush]
hubs: [disk]
probes: [disk.io, disk.hardware]
see_also: [memory.page-cache, hw-storage.block-layer]
commands: [sync]
paths: [/proc/sys/vm/dirty_expire_centisecs]
---

## TL;DR

- `write()` returning means the data reached the kernel's memory, not the
  disk. Linux writes it out later, usually within 30 seconds.
- `fsync(fd)` waits until the file's data and metadata are on the device,
  and asks the drive to flush its own cache. It's the only promise.
- To replace a file safely: write a temporary file, fsync it, rename it
  over the old one, then fsync the directory.

```
grep -E '^(Dirty|Writeback):' /proc/meminfo   # not yet on disk
sync                 # flush everything, and wait
```

## Pitfalls

- A new file isn't durable until its directory is fsynced too: after a
  crash, the data may exist but the name may not.
- Most drives have a volatile write cache; fsync flushes it. Settings
  or VMs that ignore flushes ("unsafe" cache modes) trade durability for
  speed.
- On a crash, ext4 (in its default mode) and XFS keep the filesystem
  consistent, not your file's latest contents: that's still fsync's job.
- Calling fsync after every small write is slow; batch writes, then sync
  once.

## Deep dive

### Why doesn't Linux write immediately?

Waiting lets it merge many small writes into few large ones, skip data
that gets overwritten or deleted quickly (temporary files), and let
programs continue without waiting for the disk. Writeback threads flush
data older than `vm.dirty_expire_centisecs` (30 s by default), or sooner
when dirty memory grows past `vm.dirty_background_ratio`.

### Why is rename the safe way to replace a file?

Rename is atomic: after a crash, the name points at either the old file
or the new one, never a mix. Writing in place can leave half old, half
new content. Fsync before the rename makes sure the new file's data is
there when the name moves; fsync on the directory makes the rename
itself durable. Editors and databases do exactly this.
