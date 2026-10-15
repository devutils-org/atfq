---
id: concurrency.threads
kind: concept
title: "Processes, threads and async: doing several things at once"
aliases: [thread, threads, multithreading, multithreaded, concurrency, parallelism, async, async await, gil, goroutine, goroutines, green threads, thread pool, worker threads]
hubs: []
probes: [proc.threads]
see_also: [internals.scheduler, concurrency.races, concurrency.event-loops]
commands: [ps, top]
paths: [/proc/PID/task/]
---

## TL;DR

- Processes don't share memory: safe and independent, but talking costs
  more ({{concurrency.ipc}}). Threads are tasks inside one process that
  share its memory: fast to talk, easy to break.
- Async runs many tasks on few threads: a task gives the thread back
  while it waits for the network or the disk.
- Parallel work on the CPU is capped by the number of CPUs; waiting on
  I/O isn't. Pick threads or processes for the first, async for the second.

```
ps -eLf | wc -l                 # threads on the system
ps -o nlwp,pid,comm -p PID      # threads of one process
top -H -p PID                   # one line per thread
```

## Pitfalls

- In CPython, threads don't run Python code in parallel (the GIL): use
  processes for CPU work. Free-threaded builds are changing this.
- More threads than CPUs doesn't speed up computing: it adds switching.
- Each thread reserves a stack (8 MiB of address space by default), but
  only the pages it touches use RAM.
- A crash in one thread kills the whole process; a crash in a child
  process doesn't.

## Deep dive

### What is a thread to the Linux kernel?

The same as a process: a task with its own registers, stack and
scheduling. `clone()` decides what it shares with its creator: memory,
open files, signal handlers. A thread shares almost everything; a process
from `fork()` shares nothing but copies. `top` shows tasks per process,
`/proc/PID/task/` lists them.

### How do Go and async runtimes multiplex?

They run many lightweight tasks (goroutines, futures, coroutines) on a
few kernel threads, roughly one per CPU. When a task would block, the
runtime parks it and runs another on that thread. Switching between tasks
happens in user space, far cheaper than a kernel context switch, which is
how a program can hold a hundred thousand of them.
