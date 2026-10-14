---
id: boot.cmdline
kind: concept
title: The kernel command line
aliases: [cmdline, kernel command line, kernel parameters, kernel options, /proc/cmdline, grub_cmdline_linux]
hubs: [boot]
probes: [boot.info]
see_also: [boot.sequence]
commands: [cat]
paths: [/proc/cmdline, /etc/default/grub]
---

## TL;DR

- The kernel command line is the list of options the bootloader passes to
  the kernel: which disk is root, quiet or verbose boot, driver settings.
- The current one is in `/proc/cmdline`.
- For one boot, edit the entry in the bootloader menu (`e` in GRUB). To
  keep a change, edit the bootloader's settings.

```
cat /proc/cmdline
```

## Pitfalls

- With GRUB, editing `GRUB_CMDLINE_LINUX` in `/etc/default/grub` does nothing
  until you regenerate its config: `sudo update-grub` on Debian and Ubuntu,
  `sudo grub-mkconfig -o /boot/grub/grub.cfg` on Arch.
- A typo in `root=` makes the system unbootable: try an option once from the
  menu before making it permanent.
- `quiet` hides the boot messages: remove it to see where a boot hangs.
- `systemd.unit=rescue.target` boots into a rescue shell, for one boot.

## Deep dive

### Who reads these options?

The kernel takes the ones it knows, such as `root=` or `mitigations=`.
Options with a module name and a dot go to that module:
`i915.enable_psr=0` is for the Intel graphics driver. The kernel hands the
rest to init, and systemd reads the ones that start with `systemd.`.

### What do the common options do?

`ro` mounts root read-only at first, so it can be checked; init remounts
it read-write afterwards. `quiet` keeps the boot silent, and `splash` shows
a boot animation instead. `nomodeset` keeps the kernel's graphics drivers
off, a common workaround when a new graphics card shows a black screen.
