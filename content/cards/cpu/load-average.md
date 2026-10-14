---
id: cpu.load-average
kind: concept
title: Load average
aliases: [load, load average, loadavg, uptime, system load]
hubs: [cpu]
probes: [cpu.summary]
see_also: [proc.states, proc.nice]
commands: [uptime, nproc, top]
paths: [/proc/loadavg]
---

## TL;DR

- Load average: the number of tasks running or waiting for a CPU, plus
  those waiting on disk (D state), averaged over 1, 5 and 15 minutes.
- Compare it with the number of CPUs: on 8 CPUs, a load of 8 means fully
  busy, and 16 means tasks are queuing.
- A high load with idle CPUs points to I/O: a slow disk or network storage.

```
uptime
nproc
```

## Pitfalls

- It's not a percentage: 2.0 overloads a 1-CPU machine and barely touches a
  32-CPU one.
- Linux counts tasks waiting on disk, most other Unixes don't: the same work
  shows a higher load on Linux.
- The values decay gradually, they aren't plain averages: the 1-minute value
  still carries part of older spikes.
- To see what keeps the CPUs busy, run `top` and press `1` for one line per
  CPU.

## Deep dive

### Why does Linux count tasks waiting on disk?

Since 1993, Linux adds tasks in uninterruptible sleep to the load, so that
it shows the demand on the whole system and not only on the CPUs. A machine
stuck on a slow disk then shows a high load, which makes the problem
visible, but it also makes the number harder to read: you can't tell CPU
demand from I/O waiting without looking further.

### Load or CPU usage?

CPU usage, as `top` shows it, is the share of time the CPUs were busy. Load
is the length of the queue. At 100% CPU with a load equal to the number of
CPUs, everything runs and nothing waits; with twice that load, every task
waits about as long as it runs.
