---
id: hw-cpu.cores
kind: concept
title: Cores, threads and sockets
aliases: [hyperthreading, hyper-threading, smt, threads per core, sockets, logical cpus, logical cpu, physical cores, p-cores, e-cores, big.little, nproc, lscpu, cpu topology, isolcpus]
hubs: [cpu]
probes: [cpu.topology]
see_also: [internals.scheduler, hw-cpu.caches, hw-memory.numa]
commands: [nproc, lscpu]
paths: [/sys/devices/system/cpu/]
---

## TL;DR

- A core is a complete processor. A chip (package, or socket) holds
  several; a server board can hold several chips.
- With SMT (Intel calls it Hyper-Threading), one core runs two hardware
  threads. Each shows up as a CPU, but the two share the core's units:
  expect far less than twice the speed.
- Logical CPUs = sockets × cores × threads per core. That's what `nproc`,
  `top` and the load average count.

```
lscpu | grep -E '^(CPU\(s\)|Thread|Core|Socket)'
nproc                # CPUs this process may use
```

## Pitfalls

- A load average of 8 on "8 CPUs" that are 4 cores with SMT is a busier
  machine than it looks ({{cpu.load-average}}).
- In a container limited with `--cpus=2`, `nproc` still shows every CPU
  of the host: the limit is a time quota, not fewer CPUs.
- Recent Intel chips mix performance (P) and efficiency (E) cores, like
  ARM's big.LITTLE: the same task runs at different speeds depending on
  where the scheduler puts it.
- Benchmarks vary between runs partly because of this: pin them with
  `taskset -c 2 ./bench`.

## Deep dive

### What does SMT actually gain?

A core has many execution units, and one thread rarely keeps them all
busy: it waits for memory, or mispredicts a branch. A second thread fills
the gaps. The gain is often 10 to 30 %, sometimes nothing when both
threads compete for the same units or cache. Some recent Intel chips
dropped it on their performance cores, and some clouds disable it, since
two threads on one core can spy on each other ({{hw-cpu.speculation}}).

### Why would some CPUs be isolated?

`isolcpus=` on the kernel command line, or a CPU set in a cgroup, keeps
the scheduler from placing ordinary tasks there. Real-time and trading
systems, or virtual machine hosts, then pin one task to those CPUs, so
nothing interrupts it and its caches stay warm.
