---
id: boot
kind: hub
title: Boot
aliases: [startup, booting]
probes: [boot.info]
commands:
  - { cmd: "systemd-analyze", desc: "how long the last boot took, by stage", card: boot.sequence }
  - { cmd: "systemd-analyze blame", desc: "services, slowest first" }
  - { cmd: "journalctl -b -p err", desc: "errors logged since this boot", card: systemd.failing }
  - { cmd: "systemctl get-default", desc: "the target the system boots into", card: systemd.units }
  - { cmd: "efibootmgr", desc: "UEFI boot entries and their order" }
  - { cmd: "uname -r", desc: "the running kernel" }
paths:
  - { path: "/proc/cmdline", desc: "options the kernel was started with" }
  - { path: "/boot", desc: "kernels and initramfs images" }
  - { path: "/etc/default/grub", desc: "GRUB settings, kernel options included" }
cards: []
---

From power button to login prompt: firmware, bootloader, kernel, init.
