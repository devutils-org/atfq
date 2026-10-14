---
id: boot.sequence
kind: concept
title: From firmware to init
aliases: [boot sequence, boot process, bootloader, grub, systemd-boot, initramfs, initrd, uefi, bios, secure boot]
hubs: [boot]
probes: [boot.info]
see_also: [boot.cmdline, systemd.units, kernel.modules]
commands: [systemd-analyze, journalctl, efibootmgr]
paths: [/boot, /boot/efi]
---

## TL;DR

- The firmware (UEFI or BIOS) checks the hardware and starts a bootloader
  from disk.
- The bootloader (GRUB, systemd-boot) loads the kernel and the initramfs
  into RAM.
- The kernel uses the initramfs to find and mount the real root
  filesystem, then starts init: process 1.
- Init (systemd on most distros) starts services until it reaches the
  default target.

```
systemd-analyze          # time spent in each stage
systemd-analyze blame    # slowest services first
journalctl -b -1         # logs of the previous boot
```

## Pitfalls

- `journalctl -b -1` needs a persistent journal (`/var/log/journal`
  exists). Otherwise the logs of earlier boots are gone.
- With Secure Boot on, only signed bootloaders, kernels and modules load: a
  module built on your machine (NVIDIA through DKMS, for one) has to be
  signed with a key you enroll.
- A broken initramfs drops you into an emergency shell before root is
  mounted. Rebuild it from a live USB with your distro's tool:
  `update-initramfs`, `dracut` or `mkinitcpio`.

## Deep dive

### What is the initramfs for?

The kernel can't carry drivers for every disk controller, RAID, LVM or
encryption setup. The initramfs is a small archive, loaded next to the
kernel, with just enough tools to reach the real root: unlock a LUKS disk,
assemble a RAID, mount the filesystem. Then it hands over to the real init.

### UEFI or BIOS?

A BIOS runs the 446 bytes of code at the start of the first disk, which
then loads the rest of the bootloader. UEFI reads files from the EFI System
Partition, a small FAT partition often mounted at `/boot/efi` or `/efi`,
and keeps its boot entries in firmware memory: `efibootmgr` lists them.
UEFI is also what makes Secure Boot possible.
