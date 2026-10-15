---
id: internals.exec
kind: concept
title: "How a program starts: fork and exec"
aliases: [fork, exec, execve, clone, process creation, copy on write, how a program starts]
hubs: [processes]
probes: [kernel.overview]
see_also: [bin.elf, proc.states, shell.redirections]
commands: [pstree, strace]
paths: []
---

## TL;DR

- A new process starts as a copy of an existing one: `fork()` duplicates
  the process that calls it.
- `exec()` then replaces the copy's program with another: the kernel loads
  the new ELF file and its loader ({{bin.elf}}).
- Your shell does both for every command you type. Every process descends
  from PID 1.

```
pstree -p $$       # your shell and what it started
strace -f -e trace=clone,clone3,execve bash -c 'ls; true'
```

## Pitfalls

- `fork` copies no memory up front: pages are shared until one side writes
  to them (copy-on-write), so forking even a big process is cheap.
- After `exec`, the PID stays the same: it's the same process running
  another program.
- Environment variables and open files survive `exec`: that's how
  `export` and redirections reach the program ({{shell.redirections}}).
- A parent that never waits for its children leaves zombies
  ({{proc.states}}).

## Deep dive

### Why two steps instead of one?

Between `fork` and `exec`, the child still runs the parent's code, so it
can prepare its own setup: open a file as its output, change folder, drop
privileges, set limits. Then it becomes the new program, which inherits
all of that. That's how the shell handles `cmd > file` without `cmd`
knowing anything about it.

### What does the kernel do in exec?

It reads the start of the file. For an ELF program, it builds a fresh
address space, maps the program's code and data into it, also maps the
dynamic loader when the program needs one, puts the arguments and the
environment on a new stack, and jumps to the entry point. For a file
starting with `#!`, it runs the interpreter named there instead.
