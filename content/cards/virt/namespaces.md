---
id: virt.namespaces
kind: concept
title: "Namespaces: a private view of the system"
aliases: [namespace, namespaces, unshare, nsenter, lsns, pid namespace, network namespace, mount namespace, user namespace, ip netns]
hubs: []
probes: [virt.detect]
see_also: [virt.containers, virt.rootless]
commands: [lsns, unshare, nsenter]
paths: [/proc/self/ns/]
---

## TL;DR

- A namespace gives a process its own copy of one global thing: process
  IDs (pid), mounts (mnt), network (net), hostname (uts), IPC, users,
  cgroups, clocks (time).
- A container is mostly a process with new namespaces of every kind: it
  sees only its own processes, files and network.
- Each process's namespaces are links in `/proc/PID/ns/`; two processes
  share one when the numbers match.

```
lsns                                   # namespaces and their processes
sudo unshare --pid --fork --mount-proc bash   # then ps: bash is PID 1
sudo nsenter -t PID -n ip addr         # run a command in PID's network
```

## Pitfalls

- A process in a PID namespace has two PIDs: its own inside, and another
  as the host sees it. Use the host's with `kill` from outside.
- A new network namespace has only a down loopback interface: containers
  get a virtual cable (veth) to a bridge on the host.
- Ubuntu 24.04 and later restrict unprivileged user namespaces through
  AppArmor: rootless tools may need a profile there.

## Deep dive

### How do namespaces come about?

With `clone()` or `unshare()` and flags like `CLONE_NEWPID` or
`CLONE_NEWNET`, a process asks the kernel for new namespaces for itself
or its children; `setns()` joins existing ones, which is what `nsenter`
and `docker exec` do. A namespace lives as long as a process or a bind
mount keeps it.

### Why are user namespaces special?

They map user IDs: root (0) inside can be an ordinary user outside. That
lets an unprivileged user create the other namespaces and run rootless
containers ({{virt.rootless}}), and limits the damage if something
escapes. They also widened the kernel's attack surface, which is why some
distros restrict them.
