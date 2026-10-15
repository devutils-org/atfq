---
id: hw-storage.block-layer
kind: concept
title: "From write() to the disk: the block layer"
aliases: [block layer, block device, block devices, io scheduler, i/o scheduler, mq-deadline, bfq, kyber, blk-mq, request queue, o_direct, direct io]
hubs: [disk]
probes: [disk.hardware]
see_also: [memory.page-cache, hw-storage.durability, dev.udev]
commands: [cat, lsblk]
paths: [/sys/block/DEV/queue/scheduler]
---

## TL;DR

- A `write()` goes into the page cache and returns. Later, the filesystem
  turns dirty pages into block requests ("write these sectors").
- The block layer merges neighbouring requests, orders them through an
  I/O scheduler, and hands them to the driver. The device signals each
  completion with an interrupt.
- Each disk has its scheduler: `none` for NVMe, which sorts nothing
  itself; `mq-deadline` or `bfq` for slower disks.

```
cat /sys/block/nvme0n1/queue/scheduler     # the active one in [brackets]
lsblk -t                                   # block sizes, scheduler
```

## Pitfalls

- `bfq` keeps a desktop responsive while something copies in the
  background, at some CPU cost; on fast NVMe, `none` is usually best.
- Changing the scheduler with `echo` lasts until reboot; a udev rule
  makes it permanent ({{dev.udev}}).
- `O_DIRECT` skips the page cache, for databases that cache by themselves.
  It needs aligned buffers, and it isn't a durability promise: only fsync
  is ({{hw-storage.durability}}).

## Deep dive

### What's in between, layer by layer?

Your program calls `write()`; the VFS finds the file's filesystem; the
filesystem (ext4, btrfs, XFS) copies the data into the page cache and
decides where it goes on the device. At writeback, it builds `bio`
structures: a device, a sector, memory pages. Below, device-mapper layers
(LVM, LUKS encryption) may remap or encrypt them, then blk-mq queues them
for the driver: NVMe, SCSI for SATA and USB, or virtio in a VM.

### Why do schedulers matter less than they did?

On a hard drive, sorting requests by position saved whole seeks: the
elevator algorithms. An SSD has no head to move and many internal queues,
and an NVMe drive has queues per CPU. Sorting costs more than it saves,
so the main remaining job is fairness between processes, which is what
`bfq` and cgroup I/O limits are for.
