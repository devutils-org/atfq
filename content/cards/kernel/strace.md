---
id: kernel.strace
kind: concept
title: syscalls and strace
aliases: [strace, syscall, syscalls, system call, system calls, ptrace, ptrace_scope]
hubs: []
probes: [kernel.sysctl]
see_also: [proc.signals]
commands: [strace]
paths: []
---

## TL;DR

- Programs don't touch files, the network or devices themselves: they ask
  the kernel, with system calls (`openat`, `read`, `connect`…).
- `strace` shows every system call a program makes, with its arguments and
  result. Errors such as `ENOENT` (no such file) stand out.
- `-f` follows child processes, `-e` filters, `-p` attaches to a running
  process.

```
strace -f -e trace=openat ./myapp 2>&1 | grep ENOENT
strace -c ls               # count calls, summary at the end
sudo strace -p 1234
```

## Pitfalls

- strace slows the traced program down a lot: don't leave it attached to a
  busy production service.
- `strace -p` on your own process may fail: with
  `kernel.yama.ptrace_scope = 1` (Ubuntu's default), you may only trace your
  own child processes. Use sudo.
- strace writes to stderr: add `2>&1` before a pipe, or `-o trace.txt` to
  write a file.
- A failing call isn't always a problem: programs often try several paths
  and use the first that exists.

## Deep dive

### What is a system call?

The way a program asks the kernel for something it can't do alone: open a
file, send a packet, start a process. The program puts the call number and
arguments in CPU registers, and a special instruction switches to the
kernel, which does the work and returns a result. Library functions such
as `fopen` or `printf` end up calling them.

### How does strace work?

Through `ptrace`, the same mechanism debuggers use: the kernel stops the
traced program at every system call and lets strace read its registers.
That's why it's slow. It's also why Yama restricts it: a tool that can
trace your programs can read their memory, passwords included.
