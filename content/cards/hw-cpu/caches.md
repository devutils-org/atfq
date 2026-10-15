---
id: hw-cpu.caches
kind: concept
title: "CPU caches: why memory layout matters"
aliases: [cpu cache, cpu caches, l1 cache, l2 cache, l3 cache, cache line, cache lines, cache miss, cache misses, false sharing, locality]
hubs: [cpu]
probes: [cpu.caches]
see_also: [hw-memory.hierarchy, hw-cpu.cores]
commands: [lscpu, perf]
paths: [/sys/devices/system/cpu/cpu0/cache/]
---

## TL;DR

- RAM is about 100 times slower than the CPU's own registers. Small, fast
  memories in the chip keep recent data close: L1 (tens of KiB per core,
  a few cycles), L2 (MiB, about a dozen cycles), L3 (shared, tens of
  cycles).
- Memory moves in cache lines, 64 bytes on most CPUs: reading one byte
  brings its neighbours along.
- So data read in order, packed together, is fast; data scattered in
  memory (linked lists, pointers to pointers) is slow.

```
lscpu --caches
perf stat -e cache-misses,cache-references ./program
```

## Pitfalls

- Going through a large 2D array column by column instead of row by row
  can be several times slower: each access lands on a new line.
- False sharing: two threads writing different variables that sit on the
  same line make that line bounce between their cores. Pad or separate
  hot per-thread data.
- A benchmark whose data fits in cache says little about the same code on
  data ten times bigger.

## Deep dive

### Who decides what stays in the cache?

The hardware, by itself, invisibly to programs. When a cache is full,
the least recently used line goes. The CPU also prefetches: when it sees
you read lines in order, it fetches the next ones before you ask. That's
why a sequential scan of memory runs close to full speed, and random
access doesn't.

### How do cores agree on memory?

Each core has its own L1 and L2, so two could hold different copies of
the same line. A coherence protocol prevents it: before a core writes a
line, every other copy is invalidated. That traffic is the hidden cost
of sharing data between threads, and why atomic counters updated by many
threads scale badly.
