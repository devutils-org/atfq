---
id: internals.scheduler
kind: concept
title: How the kernel shares the CPUs
aliases: [scheduler, scheduling, context switch, context switches, eevdf, cfs, preemption, time slice, run queue]
hubs: [cpu]
probes: [kernel.overview]
see_also: [proc.nice, cpu.load-average, internals.interrupts]
commands: [vmstat, top]
paths: [/proc/PID/status]
---

## TL;DR

- There are more tasks wanting a CPU than CPUs, so the scheduler gives
  each a short turn, then switches: a context switch.
- Linux preempts: it can take a CPU back from a task at any moment, so
  that nothing hogs the machine.
- Since kernel 6.6 the policy is EEVDF: tasks get CPU time in proportion
  to their weight (their niceness, {{proc.nice}}).

```
vmstat 1                      # cs: switches per second, r: tasks waiting
grep ctxt /proc/self/status   # switches of one process
```

## Pitfalls

- Thousands of context switches per second are normal. A sudden jump
  often means threads fighting over a lock, or waking up too often.
- Voluntary switches (the task waits for something) are harmless; many
  involuntary ones mean the task wanted more CPU than it got.
- More threads than CPUs doesn't make CPU-bound work faster: they only
  take turns.
- In a VM, `st` (steal) in `top` is CPU time the host gave to someone
  else while your task was ready.

## Deep dive

### What happens in a context switch?

The kernel saves the running task's registers, picks the next task, loads
its registers and, if it belongs to another process, switches to that
process's memory map. The new task carries on exactly where it stopped. A
switch happens when a task blocks (waits for a disk, a lock, a timer),
when its turn is over, or when a more urgent task wakes up.

### What did EEVDF replace?

CFS, the Completely Fair Scheduler (kernel 2.6.23, 2007), always ran the
task that had received the least CPU time. EEVDF, "earliest eligible
virtual deadline first", keeps that fairness and adds a deadline to each
task, so short, latency-sensitive tasks get to run sooner without special
tuning.
