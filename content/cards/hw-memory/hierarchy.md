---
id: hw-memory.hierarchy
kind: concept
title: RAM and the memory hierarchy
aliases: [memory hierarchy, dram, ddr4, ddr5, memory latency, memory bandwidth, latency numbers, memory channels, dual channel, dimm, dimms, memory speed]
hubs: [memory]
probes: [cpu.caches, mem.summary]
see_also: [hw-cpu.caches, internals.vm, memory.page-cache]
commands: [dmidecode, lsmem]
paths: [/proc/meminfo]
---

## TL;DR

- Every level trades size for speed. Rough orders of magnitude:
  registers under 1 ns, L1 1 ns, L2 4 ns, L3 15 ns, RAM 100 ns,
  NVMe SSD tens of µs, network to another continent 100 ms.
- RAM (DRAM) keeps each bit in a tiny capacitor that leaks: it must be
  refreshed constantly, and it loses everything when powered off.
- Bandwidth comes from channels: two modules in the right slots work in
  parallel and double it; one module alone halves it.

```
sudo dmidecode -t memory | grep -E 'Size|Type:|Speed|Locator'
lsmem                     # memory blocks the kernel sees
```

## Pitfalls

- Latency hardly improved in 20 years; bandwidth did. Code that chases
  pointers waits on latency, code that streams arrays enjoys bandwidth.
- The rated speed (DDR5-6000) often needs a profile (XMP or EXPO) turned
  on in the firmware; otherwise the RAM runs at a safer default.
- DDR5's "on-die ECC" only protects inside the chip: it isn't ECC memory
  and reports nothing ({{hw-memory.ecc}}).
- "Free" RAM is wasted RAM: Linux fills it with cache
  ({{memory.free-vs-available}}).

## Deep dive

### Why is RAM so much slower than the CPU?

Distance and design. RAM sits centimetres away, on another chip, behind
a memory controller; DRAM must open a row of cells, read it into a
buffer, then send the bytes over the bus. Caches are SRAM inside the
chip: much faster, but each bit takes six transistors instead of one
capacitor, so there can't be gigabytes of it.

### Where do swap and the page cache fit?

They extend the hierarchy downward. The page cache keeps file data in
RAM so the disk is read once ({{memory.page-cache}}); swap moves rarely
used memory to disk to free RAM ({{memory.swap}}). Each step down is
about a thousand times slower, which is why a machine that swaps heavily
crawls.
