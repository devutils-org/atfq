---
id: concurrency.limits
kind: concept
title: "Too many open files: limits on processes"
aliases: [too many open files, ulimit, ulimit -n, emfile, nofile, limitnofile, rlimit, rlimits, resource temporarily unavailable, pid_max, threads-max, tasksmax, /proc/pid/limits, fork failed]
hubs: []
probes: [proc.limits, proc.threads]
see_also: [concurrency.event-loops, systemd.overrides, proc.cgroups]
commands: [ulimit, prlimit, ls]
paths: [/proc/PID/limits, /etc/security/limits.conf]
---

## TL;DR

- Each process inherits limits from its parent: open files, processes,
  stack size… Each has a soft value (in force) and a hard ceiling the
  process may raise it to.
- The usual one to hit is open files: soft 1024 by default, while a busy
  server holds a descriptor per connection and per file.
- Services started by systemd take their limits from the unit
  (`LimitNOFILE=`), not from your shell or limits.conf.

```
ulimit -n                     # your soft limit on open files
cat /proc/PID/limits          # a running process's limits
ls /proc/PID/fd | wc -l       # how many files it has open now
```

## Pitfalls

- "Too many open files" can also be a leak: if the count in
  `/proc/PID/fd` keeps growing, a file or socket isn't being closed.
- `ulimit -n 65536` lasts for that shell and what it starts; for a
  service, `systemctl edit` and `LimitNOFILE=65536` ({{systemd.overrides}}).
- "fork: Resource temporarily unavailable" means a process or thread
  limit: `ulimit -u`, or a cgroup's `pids.max` (systemd's `TasksMax=`).
- Containers have their own limits: `docker run --ulimit nofile=…`.

## Deep dive

### Why is the default still 1024?

The old `select()` call can't handle descriptor numbers of 1024 and
above. A program using it with more descriptors would corrupt memory, so
the soft limit stays at 1024 for safety, while systemd sets the hard limit
to 524288: programs that use `epoll` raise their own soft limit at start.

### Where do limits come from at login?

The kernel's defaults, then PAM's `pam_limits` reads
`/etc/security/limits.conf` and `limits.d/` for login sessions, then
systemd's settings for your user session. Each process can lower its
limits, or raise the soft one up to the hard ceiling; only root raises
the ceiling.
