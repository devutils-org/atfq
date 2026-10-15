---
id: hw-storage.flash
kind: concept
title: "Inside an SSD: flash, TRIM and wear"
aliases: [trim, fstrim, discard, ssd wear, ssd health, tbw, write amplification, slc cache, qlc, tlc, nand, smartctl, smart, nvme smart-log]
hubs: [disk]
probes: [disk.hardware]
see_also: [hw-storage.devices, fs.fstab]
commands: [fstrim, smartctl, nvme, systemctl]
paths: [/sys/block/DEV/queue/discard_max_bytes]
---

## TL;DR

- Flash can't overwrite in place: it writes pages of a few KiB, but only
  erases whole blocks of many pages. The SSD's controller writes new data
  elsewhere and cleans up old blocks in the background.
- TRIM tells the SSD which blocks the filesystem freed, so it doesn't
  copy dead data around. Run it regularly: `fstrim.timer` does it weekly.
- Each cell survives a limited number of erase cycles. Drives are rated in
  terabytes written (TBW); health tools say how much is used.

```
systemctl status fstrim.timer
sudo smartctl -a /dev/nvme0           # look for "Percentage Used"
```

## Pitfalls

- A nearly full SSD slows down and wears faster: the controller has few
  free blocks to work with. Keep some space free.
- Cheap QLC drives write fast only until their SLC cache fills; a long
  copy can then drop to hard drive speeds.
- TRIM through LUKS encryption needs `discard` in `/etc/crypttab`. It
  reveals which blocks are unused, a small privacy trade-off.
- Deleted files on an SSD are often unrecoverable soon after TRIM: keep
  backups rather than counting on undelete tools.

## Deep dive

### What does the controller do?

It runs a flash translation layer: a map from the block numbers the
system uses to the flash pages that hold them. A rewrite goes to a fresh
page and the old one is marked stale. Garbage collection later copies the
live pages out of a mostly stale block and erases it. Those extra copies
are write amplification; TRIM and free space reduce them. The controller
also spreads writes evenly over all cells (wear levelling).

### Continuous discard or periodic TRIM?

The `discard` mount option sends a TRIM on every delete, which can slow
heavy deletes on some drives. A periodic `fstrim` batches them, and is
what most distros set up. Btrfs, since Linux 6.2, uses an asynchronous
discard by default on SSDs, which batches them itself.
