---
id: fs.tmp
kind: concept
title: /tmp vs /var/tmp
aliases: [tmp, /tmp, /var/tmp, tmpfs, mktemp, temporary files]
hubs: [disk]
probes: [fs.tmp]
see_also: [perms.special, fs.hierarchy]
commands: [mktemp]
paths: [/tmp, /var/tmp]
---

## TL;DR

- `/tmp`: short-lived scratch files. Often in RAM (tmpfs) and emptied at
  every boot.
- `/var/tmp`: temporary files that must survive a reboot. Always on disk.
- Both are cleaned by age. With systemd's default settings, files unused
  for 10 days leave `/tmp`, and 30 days for `/var/tmp`.
- In scripts, create temporary files with `mktemp`, never with a fixed name.

```
mktemp        # a new private file in /tmp
mktemp -d     # a new private folder
```

## Pitfalls

- A tmpfs `/tmp` lives in RAM: a huge file there competes with your apps
  for memory. tmpfs is capped, at half the RAM unless set otherwise.
- A fixed name like `/tmp/myapp.log` is a security hole: another user can
  create that name first, as a link to one of your files.
- Everyone can write to `/tmp`, yet nobody can delete other people's files
  there: that's the sticky bit (see {{perms.special}}).
- Don't keep anything you need in `/tmp`: it may be gone after a reboot.

## Deep dive

### Why are there two temporary folders?

The Filesystem Hierarchy Standard gives them different promises: `/tmp` may
be cleared at boot, `/var/tmp` is kept between reboots. Programs that need
large or long-lived temporary data, such as a package build or a text
editor's recovery files, use `/var/tmp`.

### Why did distros move /tmp to RAM?

RAM is faster than any disk, writes to RAM don't wear out an SSD, and
emptying it at boot is free. Fedora made a tmpfs `/tmp` the default in
release 18 (2013), and Debian in release 13 (2025).

The cost is memory: big files in `/tmp` take RAM. tmpfs can move them to
swap under pressure, but a machine without swap has nowhere to put them.
