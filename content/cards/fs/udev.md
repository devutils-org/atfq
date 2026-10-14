---
id: dev.udev
kind: concept
title: /dev and udev
aliases: [udev, /dev, devices, device files, udevadm, by-id, by-uuid, udev rules]
hubs: [disk]
probes: [dev.udev]
see_also: [fs.fstab, kernel.modules, perms.sudo-groups]
commands: [udevadm, lsblk]
paths: [/dev, /dev/disk/by-id/, /etc/udev/rules.d/]
---

## TL;DR

- `/dev` holds device files: `/dev/sda` (a disk), `/dev/ttyUSB0` (a serial
  adapter), `/dev/null`. Programs open them like files to talk to drivers.
- udev names them when hardware appears, sets their owner and rights, and
  runs actions, following rules.
- `/dev/disk/by-id/` and `/dev/disk/by-uuid/` hold names that stay the same
  from one boot to the next.

```
ls -l /dev/disk/by-id/
udevadm monitor          # plug something in, watch the events
udevadm info /dev/sda
```

## Pitfalls

- Your own rules go in `/etc/udev/rules.d/`, named like
  `70-keyboard.rules`. A file there with the same name as a packaged rule
  replaces it.
- After editing a rule, run `sudo udevadm control --reload`, then unplug
  and plug the device back (or `sudo udevadm trigger`).
- "Permission denied" on a serial port or a USB device: join its group
  (`dialout` or `uucp`, `plugdev`) instead of a `chmod 666` that udev undoes
  at the next plug.
- `/dev/sda` and `/dev/sdb` can swap places: scripts should use
  `/dev/disk/by-id/`.

## Deep dive

### Character or block device?

The first letter in `ls -l` says it. `b`: a block device, such as a disk,
read in blocks through the page cache. `c`: a character device, such as a
terminal, a serial port or `/dev/null`, a stream of bytes. Each has a
major:minor number that names its driver and instance, the same numbers as
in `/sys/block/*/dev`.

### Where does /dev come from?

It's a devtmpfs filesystem that the kernel fills itself as devices appear.
udev then fixes owners and rights, adds the stable links, and runs actions.
Before devtmpfs (kernel 2.6.32, 2009), udev created every file itself; and
before udev, `/dev` was a static folder with thousands of entries.
