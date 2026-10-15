---
id: boot.targets
kind: concept
title: systemd targets
aliases: [target, targets, runlevel, default target, multi-user.target, graphical.target, rescue mode, emergency mode]
hubs: [boot]
probes: [boot.info]
see_also: [systemd.units, boot.cmdline]
commands: [systemctl]
paths: [/etc/systemd/system/default.target]
---

## TL;DR

- A target is a stage the system can reach. `multi-user.target`: text mode
  with network and services. `graphical.target`: the same plus a login
  screen.
- Boot stops at the default target: `systemctl get-default` shows it,
  `set-default` changes it.
- `rescue.target` and `emergency.target` are minimal modes for repairs.

```
systemctl get-default
sudo systemctl set-default multi-user.target
sudo systemctl isolate rescue.target   # switch now
```

## Pitfalls

- `isolate` stops every unit the new target doesn't need: from a desktop,
  going to `multi-user.target` closes your session.
- For a single boot, add `systemd.unit=rescue.target` to the kernel
  command line from the boot menu (see {{boot.cmdline}}).
- Rescue and emergency modes ask for the root password. Where root is
  locked (Ubuntu), they can't open a shell.

## Deep dive

### How do targets replace runlevels?

SysV init had numbered runlevels, 0 to 6, with fixed meanings. Targets
have names, depend on each other and can be combined. Aliases such as
`runlevel3.target` (multi-user) and `runlevel5.target` (graphical) keep old
habits working.

### Rescue or emergency?

`rescue.target` mounts the filesystems, starts a few basic services and
opens a root shell. `emergency.target` mounts only the root filesystem,
read-only, for when even `/etc/fstab` is broken.
