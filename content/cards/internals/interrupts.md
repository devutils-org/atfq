---
id: internals.interrupts
kind: concept
title: "Interrupts: how devices get attention"
aliases: [interrupt, interrupts, irq, irqs, softirq, softirqs, ksoftirqd, timer tick, irqbalance]
hubs: []
probes: [kernel.irq]
see_also: [internals.scheduler, kernel.dmesg]
commands: [watch]
paths: [/proc/interrupts, /proc/softirqs]
---

## TL;DR

- A device that needs the CPU (a packet arrived, a disk finished, a key
  was pressed) raises an interrupt: the CPU stops what it's doing and runs
  the kernel's handler for that device.
- The handler does only the urgent part; the rest runs a moment later, as
  a software interrupt (softirq) or in a kernel thread.
- The timer interrupt is the kernel's heartbeat: it's how the scheduler
  gets the CPU back.

```
cat /proc/interrupts
watch -d -n1 cat /proc/softirqs      # -d highlights what changes
```

## Pitfalls

- The counters only grow from boot: compare two readings, or watch them.
- One CPU taking every interrupt of a busy network card can become the
  bottleneck. Multi-queue cards and `irqbalance` spread them over CPUs.
- `ksoftirqd` using CPU means deferred work piled up, often under heavy
  network traffic.

## Deep dive

### Why split the work in two?

While a hardware handler runs, other interrupts may have to wait, so it
must be quick: acknowledge the device, take the data. The longer work,
such as carrying a network packet up the stack, runs right after as a
softirq, or in a kernel thread, with interrupts enabled again.

### Does the kernel tick all the time?

It used to, 100 to 1000 times per second on every CPU, commonly 250 or
1000 depending on the distro. Current kernels stop the tick on idle CPUs
to save power ("tickless"), and can even stop it on a busy CPU reserved
for a single task (`nohz_full`), for latency-sensitive work.
