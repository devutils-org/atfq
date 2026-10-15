---
id: kernel.sysctl
kind: concept
title: sysctl
aliases: [sysctl, sysctl.conf, sysctl.d, /proc/sys, tunables, kernel tunables, kernel settings]
hubs: []
probes: [kernel.sysctl]
see_also: [fs.proc-sys, memory.swap]
commands: [sysctl]
paths: [/proc/sys, /etc/sysctl.d/]
---

## TL;DR

- Kernel settings are files under `/proc/sys`. `sysctl` reads and writes them
  with dots instead of slashes: `vm.swappiness` is
  `/proc/sys/vm/swappiness`.
- `sysctl -w` changes a setting until the next reboot.
- To keep a change, write it in a `.conf` file in `/etc/sysctl.d/`: it is
  applied at every boot.

```
sysctl vm.swappiness
sudo sysctl -w vm.swappiness=10
sudo sysctl --system      # apply every file now
```

## Pitfalls

- Files from `/etc/sysctl.d/`, `/run/sysctl.d/` and `/usr/lib/sysctl.d/` are
  applied in file name order, and the last value wins: give yours a high
  number, like `90-swap.conf`.
- A setting for a module that isn't loaded yet (such as `net.bridge.*`)
  fails at boot: the file is read before the module loads.
- Inside a container, many settings are read-only, or only change the
  container.
- Tuning lists copied from the web often do nothing, or harm: change one
  setting at a time, for a reason you can measure.

## Deep dive

### Why are kernel settings files?

Exposing them as files in `/proc/sys` lets any tool read and change them,
from `cat` to a configuration manager. `sysctl` is a convenience on top.
There used to be a `sysctl()` system call as well; it was rarely used and
was removed in kernel 5.5.

### Which settings are worth knowing?

`vm.swappiness` (see {{memory.swap}}) and `vm.overcommit_memory` ({{memory.oom-killer}}) for memory.
`kernel.dmesg_restrict` ({{kernel.dmesg}}) and `kernel.yama.ptrace_scope` ({{kernel.strace}}) for
what regular users may inspect. `net.ipv4.ip_forward` lets the machine
route packets, which containers and VPN servers need.
`fs.inotify.max_user_watches` is the one to raise when an editor or a build
tool runs out of file watches.
