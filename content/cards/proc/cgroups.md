---
id: proc.cgroups
kind: concept
title: cgroups
aliases: [cgroup, cgroups, control groups, systemd-cgls, systemd-cgtop, memory limit, slice, slices]
hubs: [processes]
probes: [proc.cgroup]
see_also: [memory.oom-killer, systemd.units]
commands: [systemd-cgls, systemd-cgtop, systemctl]
paths: [/sys/fs/cgroup, /proc/self/cgroup]
---

## TL;DR

- A cgroup is a group of processes that the kernel measures and limits
  together: CPU, memory, disk I/O, number of processes.
- systemd puts every service and every login session in a cgroup of its
  own, such as `system.slice/nginx.service`.
- Containers are built on them: a container's memory limit is a cgroup
  limit.

```
systemd-cgls                                   # the tree
systemd-cgtop                                  # usage per cgroup, live
sudo systemctl set-property nginx.service MemoryMax=1G
```

## Pitfalls

- A process that reaches its cgroup's memory limit is OOM-killed, even when
  the machine has free RAM (see 5.2).
- `systemctl set-property` is permanent: add `--runtime` for a change that
  ends at the next reboot.
- Killing a service's main process can leave its children running;
  `systemctl stop` stops the whole cgroup.
- Most current distros use cgroup v2 only; some old container tools still
  expect v1.

## Deep dive

### What changed with cgroup v2?

Version 1 had one tree per kind of resource, each organized its own way,
which made limits hard to combine. Version 2, stable since kernel 4.5
(2016), has a single tree for all of them. Fedora made it the default in
release 31 (2019); Debian 11 and Ubuntu 21.10 followed in 2021.

### How does systemd use them?

The top level is split into slices: `system.slice` for services,
`user.slice` for sessions, `machine.slice` for VMs and containers. Each
service gets its own cgroup, so stopping it ends every process it started,
and no process escapes by forking. Unit settings such as `MemoryMax=`,
`CPUWeight=` and `TasksMax=` set the limits.
