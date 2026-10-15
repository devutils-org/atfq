---
id: proc.nice
kind: concept
title: nice and priorities
aliases: [nice, renice, niceness, priority, priorities, ionice]
hubs: [cpu]
probes: []
see_also: [cpu.load-average]
commands: [nice, renice, ionice, chrt]
paths: []
---

## TL;DR

- Niceness goes from -20 (greedy) to 19 (polite). The default is 0.
- A higher niceness gives a process a smaller share of the CPUs when they
  are busy. When they're idle, it changes nothing.
- Anyone can make their own process nicer; making it greedier needs root.

```
nice -n 10 make -j8          # start a build at low priority
renice -n 5 -p 1234          # change a running process
ionice -c3 rsync -a src/ dst/  # use the disk only when idle
```

## Pitfalls

- Niceness only shares the CPU: a nice process can still fill the RAM or
  keep the disk busy.
- `ionice` only has an effect with an I/O scheduler that honors priorities,
  such as BFQ: check `/sys/block/DEV/queue/scheduler`.
- In `top`, NI is the niceness and PR the kernel's priority: 20 + niceness
  for normal processes.

## Deep dive

### How does the scheduler use niceness?

Each niceness level changes a task's weight by about 25%. Two busy tasks one
level apart share a CPU roughly 55% to 45%; ten levels apart, the nicer one
gets about a tenth of the other's time.

Since kernel 6.6 the scheduler is EEVDF, which replaced CFS. The weights
work the same way; EEVDF mostly changes how quickly a task that woke up
gets to run.

### What about real-time priorities?

`chrt` puts a process in a real-time class (SCHED_FIFO or SCHED_RR). Those
always run before normal tasks, whatever their niceness, so a runaway
real-time task can take over a CPU. By default the kernel keeps 5% of each
second for normal tasks, so the system stays usable.
