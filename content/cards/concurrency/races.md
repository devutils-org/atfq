---
id: concurrency.races
kind: concept
title: Race conditions, locks and atomics
aliases: [race condition, race conditions, data race, mutex, lock, locks, deadlock, deadlocks, atomic, atomics, thread safety, thread-safe, critical section, memory ordering]
hubs: []
probes: [cpu.topology]
see_also: [concurrency.threads, hw-cpu.caches]
commands: [gdb]
paths: []
---

## TL;DR

- A race: the result depends on which thread runs first. `count += 1` is
  read, add, write: two threads can read the same value and lose one
  increment.
- A lock (mutex) lets one thread at a time into a critical section.
  Atomics do one small operation indivisibly, with no lock.
- Races hide: the code works a thousand times, then fails under load or
  on a machine with more cores.

```
gcc -fsanitize=thread -g prog.c   # ThreadSanitizer finds data races
go test -race ./...
```

## Pitfalls

- Deadlock: thread A holds lock 1 and waits for 2, thread B holds 2 and
  waits for 1. Always take locks in the same order.
- A lock protects data only if every access takes it, reads included.
- "Check then act" (if the file doesn't exist, create it) races with
  other processes too: use atomic operations (`O_EXCL`, rename).
- Holding a lock while waiting on the network or disk serialises
  everything behind it.

## Deep dive

### Why can't threads just read and write shared variables?

Compilers and CPUs reorder memory operations for speed, as long as one
thread can't tell. Another thread can: it may see a flag set before the
data it guards. Locks and atomics include barriers that forbid harmful
reorderings, and make each core's cached copy coherent at the right time
({{hw-cpu.caches}}). Without them, even "harmless" races are undefined
behaviour in C, C++ and Rust.

### How do languages help?

Rust refuses to compile data races: shared data must be behind a lock or
an atomic. Go and Java define what racy code may observe, and ship race
detectors. Message passing (channels, queues) avoids shared state
altogether: one owner per piece of data, the others send it requests.
