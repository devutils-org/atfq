---
id: systemd.units
kind: concept
title: Units and timers
aliases: [systemd, systemctl, unit, units, service, services, timer, timers, cron, crontab]
hubs: []
probes: [systemd.status]
see_also: [systemd.failing, systemd.overrides, boot.sequence]
commands: [systemctl]
paths: [/etc/systemd/system, /usr/lib/systemd/system]
---

## TL;DR

- systemd manages units: services (`.service`), timers (`.timer`), mounts,
  sockets, and targets, which group other units.
- `start` and `stop` act now. `enable` and `disable` decide what starts at
  boot. They are separate.
- A timer starts a service on a schedule: it is systemd's version of cron.

```
systemctl status nginx
sudo systemctl enable --now nginx   # start now, and at every boot
systemctl list-timers
```

## Pitfalls

- `enable` alone doesn't start the service now: add `--now`, or run
  `start` too.
- After editing a unit file, run `sudo systemctl daemon-reload`, or systemd
  keeps using the old version.
- `mask` is stronger than `disable`: a masked unit can't start at all, not
  even by hand or as a dependency.
- `systemctl --user` manages your own units, separately from the system's,
  and without sudo.

## Deep dive

### What is a target?

A target is a named group of units that marks a stage: `multi-user.target`
is a working system without a graphical login, `graphical.target` adds one.
At boot, systemd starts everything the default target needs (see 9.1).
Targets replaced the numbered runlevels of the older SysV init.

### Timers or cron?

Timers log every run in the journal, can run a job missed while the
machine was off (`Persistent=true`), and their service can be started by
hand to test it. A timer runs the service of the same name:

```
[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true
```

cron stays simpler for a one-line job, and is still available.
