---
id: virt.containers
kind: concept
title: What a container really is
aliases: [container, containers, docker, podman, docker run, containerization, oci, runc, lxc, container vs vm, pid 1 in container]
hubs: []
probes: [virt.detect]
see_also: [virt.namespaces, proc.cgroups, virt.vms]
commands: [docker, podman, uname]
paths: [/sys/fs/cgroup/, /proc/self/ns/]
---

## TL;DR

- A container is a group of ordinary processes on your kernel. Nothing
  is emulated: `uname -r` inside shows the host's kernel.
- Three kernel features make them feel apart: namespaces (what they
  see), cgroups (what they may use) and capabilities and seccomp (what
  they may do). An image gives them their own files.
- So containers start in milliseconds and cost little, but they all
  depend on the host's kernel.

```
docker run --rm -it alpine sh     # then: ps, hostname, uname -r
ps -ef --forest                   # on the host: the same processes
```

## Pitfalls

- Your program as PID 1 doesn't stop on SIGTERM unless it handles it:
  `docker stop` waits 10 seconds, then kills it. `--init` adds a tiny init.
- A container can't run a different kernel: Linux images only, and
  kernel modules or sysctls come from the host.
- On a Mac or Windows, Docker and Podman run Linux in a virtual machine,
  and the containers inside it.
- Anything written inside a container is gone when it's removed, unless
  it's in a volume ({{virt.images}}).

## Deep dive

### What does `docker run` actually do?

The engine unpacks the image's layers, stacks them as the root
filesystem, then asks a runtime (`runc` or `crun`) to start the process:
it creates new namespaces, puts the process in a cgroup with its limits,
drops capabilities, applies a seccomp filter of allowed system calls,
and calls exec. From then on it's just a process the kernel schedules
like any other.

### Container or virtual machine?

A VM runs its own kernel on emulated hardware: stronger isolation, any
operating system, more memory and a slower start. A container shares
the host's kernel: light and fast, but a kernel flaw affects them all.
Clouds often combine them: containers inside small VMs ({{virt.vms}}).
