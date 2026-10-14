---
id: boot.loaders
kind: concept
title: GRUB vs systemd-boot vs UKI
aliases: [grub, grub2, systemd-boot, bootctl, uki, unified kernel image, boot loader]
hubs: [boot]
probes: [boot.info]
see_also: [boot.sequence, boot.cmdline]
commands: [bootctl, efibootmgr]
paths: [/boot/grub, /boot/efi/EFI]
---

## TL;DR

- GRUB is the classic bootloader: menus, many filesystems, BIOS and UEFI.
  Its configuration is generated (`grub.cfg`).
- systemd-boot is a small UEFI-only menu that reads simple entry files from
  the EFI partition; `bootctl` manages it.
- A UKI (unified kernel image) bundles kernel, initramfs and command line
  into one `.efi` file, which UEFI or systemd-boot starts directly.

```
bootctl status
efibootmgr -v
ls /boot/efi/EFI
```

## Pitfalls

- Never edit `grub.cfg` by hand: it is regenerated (see 9.2).
- systemd-boot needs the kernels on a partition UEFI can read, usually the
  EFI partition: it must be big enough for several kernels.
- With a UKI, the command line is part of the signed image: changing it
  means building the image again.
- In dual boot, a Windows update may change the boot order: fix it in the
  firmware menu or with `sudo efibootmgr -o`.

## Deep dive

### Why UKIs?

With Secure Boot, the kernel is signed, but the initramfs and the command
line next to it are not, so they could be changed. A UKI puts all three in
one signed file, and the whole path from firmware to root is checked.
Tools such as `ukify`, `dracut` and `mkinitcpio` can build them.

### Which one do I have?

`bootctl status` reports the loader that started the system, when it tells
systemd. The firmware's entries, listed by `efibootmgr -v`, show the file
it starts: `shimx64.efi` or `grubx64.efi` for GRUB, `systemd-bootx64.efi`
for systemd-boot.
