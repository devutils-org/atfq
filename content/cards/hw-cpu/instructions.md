---
id: hw-cpu.instructions
kind: concept
title: "What a CPU does: instructions and architectures"
aliases: [instruction set, isa, architecture, cpu architecture, x86-64, x86_64, amd64, arm64, aarch64, arm, risc-v, riscv, avx, avx2, avx-512, neon, cpu flags, x86-64-v3, microarchitecture level]
hubs: [cpu]
probes: [cpu.features]
see_also: [hw-cpu.cores, bin.elf]
commands: [uname, lscpu, grep]
paths: [/proc/cpuinfo]
---

## TL;DR

- A CPU runs a stream of machine instructions: load, add, compare, jump,
  store. Which instructions exist, and how they're encoded, is its
  architecture: x86-64, ARM64 (aarch64) or RISC-V.
- A binary is built for one architecture: an ARM64 program can't run on
  an x86-64 CPU, unless an emulator translates it.
- Within one architecture, newer CPUs add extensions (AVX2, AVX-512 on x86;
  SVE on ARM). x86-64 groups them into levels v1 to v4.

```
uname -m                          # your architecture
grep -m1 -wo -E 'avx2|avx512f|sve' /proc/cpuinfo
```

## Pitfalls

- "Exec format error" usually means a binary for another architecture:
  `file ./program` says which ({{bin.elf}}).
- "Illegal instruction" (SIGILL): the program uses an extension this CPU
  lacks, often one built with `-march=native` on a newer machine.
- Containers share the host's CPU: an `arm64` image on an x86-64 machine
  only runs through QEMU emulation, much slower.
- Some distros now need a minimum level: RHEL 9 needs x86-64-v2, RHEL 10
  needs v3. Older CPUs can't install them.

## Deep dive

### How does a CPU run instructions so fast?

It works like an assembly line. Each instruction is fetched, decoded,
executed and its result written back, and many instructions are at
different stages at the same time: that's the pipeline. Modern cores go
further: they decode several instructions per cycle, run them out of
order as soon as their inputs are ready, and guess the outcome of
branches to keep the pipeline full ({{hw-cpu.speculation}}).

### Why are there x86-64 levels?

x86-64 dates from 2003. Compilers target the original instruction set by
default, so every program runs on every 64-bit PC, but most ignore 20
years of additions. The levels name common sets: v2 adds SSE4.2 and
POPCNT (about 2009), v3 adds AVX2 and FMA (about 2013 to 2015), v4 adds
AVX-512. Building for v3 makes some code noticeably faster, at the cost
of older machines.
