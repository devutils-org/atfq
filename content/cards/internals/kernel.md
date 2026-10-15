---
id: internals.kernel
kind: concept
title: What does the kernel do?
aliases: [kernel, linux kernel, what is the kernel, kernel space, user space, userspace, monolithic kernel]
hubs: []
probes: [kernel.overview]
see_also: [internals.scheduler, internals.vm, kernel.strace]
commands: [uname]
paths: [/proc/version]
---

## TL;DR

- The kernel is the one program with full control of the hardware. It
  shares the CPUs, the memory and the devices among all the others.
- Programs run in user space and can't touch the hardware: they ask the
  kernel, through system calls ({{kernel.strace}}).
- Linux is one big program, a monolithic kernel, that loads most drivers
  as modules when needed ({{kernel.modules}}).

```
uname -r              # the running kernel
cat /proc/version     # who built it, and with what
```

## Pitfalls

- "Linux" is only the kernel: the shell, systemd and the desktop come from
  other projects, and a distribution puts them together.
- The kernel's own threads show in `ps` between brackets: `[kswapd0]`,
  `[kworker/0:1]`. They aren't programs you can uninstall.
- A bug in a program kills that program; a bug in the kernel stops the
  whole machine (a "kernel panic"). That's the price of full control.
- A new kernel only runs after a reboot.

## Deep dive

### What separates user space from kernel space?

The CPU itself. It has privilege levels: kernel code runs at the highest
(ring 0 on x86, EL1 on ARM) and may touch any memory and any device;
programs run at the lowest (ring 3, EL0). When a program tries a privileged
instruction, or memory it doesn't own, the CPU stops it and the kernel
usually ends it, with a "segmentation fault". The only door between the two
sides is the system call.

### Why one big kernel rather than a microkernel?

In a microkernel (Minix, seL4), drivers and filesystems run as separate
programs: a bug in one can't bring the others down, but every request
crosses more boundaries. Linux keeps everything in one address space for
speed, and gets its flexibility from modules. The famous 1992 debate
between Andrew Tanenbaum and Linus Torvalds was about exactly this.

### Why is everything a file?

The kernel offers one interface, `open`, `read`, `write`, `close`, for
files, devices, pipes, sockets and its own information. A layer called the
VFS (virtual filesystem) sits under every filesystem and makes that work.
That's why `cat` reads `/proc/meminfo` and `/dev/null` as easily as a text
file.
