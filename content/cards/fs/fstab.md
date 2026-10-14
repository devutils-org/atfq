---
id: fs.fstab
kind: concept
title: fstab and mount options
aliases: [fstab, /etc/fstab, mount, mount options, umount, nofail, noatime, automount]
hubs: [disk]
probes: [fs.fstab]
see_also: [dev.udev, systemd.units]
commands: [mount, findmnt, lsblk, blkid]
paths: [/etc/fstab]
---

## TL;DR

- `/etc/fstab` lists what to mount at boot, one line per filesystem: what,
  where, type, options, then two numbers.
- Name disks by `UUID=…` (from `lsblk -f`), not `/dev/sdb1`: device names
  can change from one boot to the next.
- Test a new line before rebooting: `sudo mount -a` mounts everything
  listed that isn't mounted yet.

```
lsblk -f
sudo mount -a
sudo findmnt --verify      # checks fstab for mistakes
```

## Pitfalls

- A wrong line can stop the boot in emergency mode. Add `nofail` for disks
  that may be missing, such as USB drives and network shares.
- After editing fstab, run `sudo systemctl daemon-reload`: systemd turns
  each line into a mount unit, and keeps the old ones until then.
- Network shares (nfs, cifs) wait for the network at boot; with `nofail`
  and `x-systemd.automount`, a missing server doesn't block it.
- Most systems already mount with `relatime`, which updates access times at
  most once a day: `noatime` saves little more.

## Deep dive

### What do the two numbers at the end mean?

The fifth field was for `dump`, an old backup tool: use 0. The sixth sets
the order of filesystem checks at boot: 1 for the root filesystem, 2 for
the others, 0 to skip. btrfs and XFS check themselves, so they use 0.

### Who reads fstab today?

On systemd systems, a generator turns every line into a `.mount` unit at
boot and at each `daemon-reload`. That's why systemd options work there:
`x-systemd.automount` mounts on first access, and
`x-systemd.device-timeout=10s` limits how long boot waits for a disk.
