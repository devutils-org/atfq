---
id: memory.swap
kind: concept
title: Swap and swappiness
aliases: [swap, swappiness, zram, swap file, swapfile, swapping]
hubs: [memory]
probes: [mem.summary]
see_also: [memory.free-vs-available, memory.oom-killer]
commands: [swapon, vmstat]
paths: [/proc/swaps, /proc/sys/vm/swappiness]
---

## TL;DR

- Swap is disk space the kernel uses as overflow for RAM: app memory that
  is rarely touched moves there.
- A little swap in use is fine. Constant swapping in and out is what makes
  a machine slow.
- zram is swap in compressed RAM: no disk involved. Fedora uses it by
  default.

```
swapon --show
vmstat 1        # si and so: swapped in and out, per second
```

## Pitfalls

- No swap at all doesn't make a machine faster: the kernel can then only
  drop file cache, and the OOM killer comes sooner.
- Hibernation writes the RAM to swap: it needs a swap partition or file
  large enough for the RAM in use. zram can't be used for it.
- A swap file on btrfs needs special settings (no copy-on-write, no
  compression): create it with `btrfs filesystem mkswapfile`.
- `swapoff -a` on a busy machine pulls everything back into RAM, and can
  wake the OOM killer.

## Deep dive

### Why swap out when RAM is still free?

The kernel balances two uses of RAM: app memory and file cache. An app page
nobody touched for hours is worth less than file data in active use, so
moving it out makes the machine faster overall. `vm.swappiness` sets that
balance (see {{memory.free-vs-available}}).

### Swap partition, swap file or zram?

A partition is the classic setup. A swap file performs as well on current
kernels and is easier to resize. zram keeps compressed pages in RAM, often
at about half their size: it suits laptops and small machines, and it costs
some CPU instead of disk time. Some setups combine zram first and a disk
swap behind it.
