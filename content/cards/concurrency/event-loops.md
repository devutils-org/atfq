---
id: concurrency.event-loops
kind: concept
title: "Event loops: one thread, thousands of connections"
aliases: [event loop, epoll, io_uring, kqueue, non-blocking, non-blocking io, nonblocking, c10k, libuv, reactor, blocking call]
hubs: []
probes: [proc.limits]
see_also: [concurrency.threads, concurrency.limits, kernel.strace]
commands: [strace]
paths: []
---

## TL;DR

- A thread per connection spends most of its time waiting. An event loop
  asks the kernel "tell me when any of these sockets is ready", then
  handles whichever is, one at a time.
- On Linux that question is `epoll`. Node.js, nginx, Redis, Python's
  asyncio and Rust's tokio are built on it.
- One blocking call (a slow file read, a CPU-heavy loop) freezes every
  connection the loop serves.

```
strace -f -e trace=epoll_wait,epoll_ctl -p PID   # the loop at work
```

## Pitfalls

- Run CPU-heavy work or blocking libraries on a worker thread or process;
  async frameworks have helpers for it (`run_in_executor`, `spawn_blocking`).
- Every connection is a file descriptor: the open-files limit caps how
  many you can hold ({{concurrency.limits}}).
- Regular files are always "ready" for epoll: disk reads still block,
  which is why runtimes use threads, or io_uring, for files.

## Deep dive

### How does epoll scale?

The older `select` and `poll` take the whole list of descriptors on every
call, so the cost grows with the number of connections. With `epoll`,
the program registers each descriptor once; the kernel keeps a ready list
filled by the network stack, and `epoll_wait` returns only the descriptors
that have something to do. Ten thousand idle connections cost nothing.

### What does io_uring change?

Instead of asking "is it ready?" and then calling `read`, a program puts
whole requests in a ring shared with the kernel and collects results from
another ring: fewer system calls, and real asynchronous file I/O. It's
newer and has had security issues, so some distros and containers turn it
off.
