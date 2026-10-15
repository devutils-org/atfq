---
id: internals.vm
kind: concept
title: How virtual memory works
aliases: [virtual memory, page fault, page faults, mmu, tlb, page table, page tables, segfault, segmentation fault, huge pages, thp]
hubs: [memory]
probes: [mem.vm]
see_also: [memory.page-cache, memory.swap, internals.exec]
commands: [ps, top]
paths: [/proc/PID/maps, /proc/PID/smaps, /proc/vmstat]
---

## TL;DR

- Every process sees its own memory, as if it were alone: the addresses
  it uses are virtual.
- The CPU's MMU translates them into real RAM through page tables the
  kernel keeps, one page (usually 4 KiB) at a time.
- Memory is only handed out when first touched: the first access to a page
  causes a page fault, and the kernel maps a page then.

```
cat /proc/self/maps | head            # a process's memory map
grep -E '^pg(maj)?fault ' /proc/vmstat
```

## Pitfalls

- Page faults are normal: most are minor, served from RAM. Major faults
  read the disk, and those are the slow ones.
- VSZ (or VIRT) in `ps` and `top` is reserved address space, not RAM in
  use: look at RSS (RES).
- A segmentation fault is an access to an address the process has no
  mapping for, such as a null pointer: the kernel sends it SIGSEGV.
- Shared libraries count in the RSS of every process using them, but sit
  in RAM once: adding up RSS overstates. `PSS` in smaps splits them fairly.

## Deep dive

### Why not let programs use real addresses?

Isolation first: a process can't read or break another's memory. Then
freedom for the kernel: it can place pages anywhere, share identical ones
(libraries, or a process and its fork until one writes), move rarely used
ones to swap, and give every program the same simple layout.

### What are the TLB and huge pages?

Reading the page tables on every memory access would be slow, so the CPU
keeps recent translations in a small cache, the TLB. With 4 KiB pages, a
big program needs many entries; a huge page (2 MiB on x86-64) covers 512
times more with one. Transparent huge pages let the kernel use them by
itself: `always`, only where a program asks (`madvise`), or `never`.
