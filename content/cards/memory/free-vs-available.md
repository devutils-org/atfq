---
id: memory.free-vs-available
kind: concept
title: free vs available
aliases: [free, available, ram usage, meminfo, memory full, buff/cache]
hubs: [memory]
probes: [mem.summary]
see_also: [memory.swap, memory.oom-killer]
commands: [free, vmstat]
paths: [/proc/meminfo]
---

## TL;DR

- **free**: memory nobody is using at all, not even as cache.
- **available**: what your apps can still get without swapping.
- Look at **available**, not free: the `available` column of the `Mem:`
  line in `free -h`.

```
free -h
```

## Pitfalls

- "free" near zero is normal: Linux fills spare RAM with page cache (file
  data kept in RAM so it doesn't have to be read from disk again).
- That cache is handed back as soon as an app needs the memory.
- Not all of buff/cache can be handed back: files in tmpfs, like
  `/dev/shm`, live there too. "available" already accounts for that.
- A little swap in use is not a problem by itself. What hurts is constant
  swapping: watch the `si` and `so` columns of `vmstat 1`.

## Deep dive

### Why is unused RAM wasted RAM?

RAM is much faster than any disk. When a program reads a file, Linux keeps a
copy in the page cache, so the next read comes straight from memory. Empty
RAM does nothing for you, so the kernel fills it with cache and takes it back
when programs ask for memory.

Taking cache back is cheap when the data is unchanged: it's still on disk,
the kernel can drop it. Changed ("dirty") data must be written to disk first,
which is slower.

### How is "available" calculated?

Since kernel 3.14, the kernel computes it for you: the `MemAvailable` line in
`/proc/meminfo`, which `free` shows as "available". Roughly: free memory,
plus the page cache and kernel caches that could be reclaimed, minus a safety
margin the kernel keeps for itself. It's an estimate, not a promise.

Before that, tools added free + buffers + cache. That overestimated, because
part of the cache can't be dropped (tmpfs files, for example).

### What is swappiness and when should I change it?

`/proc/sys/vm/swappiness` (default 60) tells the kernel how to balance two
ways of freeing memory: dropping page cache, or moving app memory to swap.
Higher values make it more willing to swap. The range is 0 to 100, extended
to 200 in kernel 5.8 for fast swap like zram.

Changing it rarely fixes a machine that's short of RAM: it only changes what
gets pushed out first. To try a value until the next reboot:

```
sudo sysctl vm.swappiness=10
```

To keep it, put `vm.swappiness = 10` in a file under `/etc/sysctl.d/`.
