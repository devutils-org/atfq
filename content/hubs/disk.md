---
id: disk
kind: hub
title: Disk
aliases: [disks, storage, ssd, hdd, nvme, partitions]
probes: [disk.tree]
commands:
  - { cmd: "lsblk", desc: "tree of disks and partitions" }
  - { cmd: "df -h", desc: "free space per mounted filesystem" }
  - { cmd: "du -sh *", desc: "size of each item in this folder" }
  - { cmd: "findmnt", desc: "mount points and their options" }
  - { cmd: "find / -xdev -type f -size +1G 2>/dev/null", desc: "files over 1 GiB on the root filesystem" }
  - { cmd: "iostat -x 1", desc: "live disk activity (package sysstat)" }
  - { cmd: "sudo smartctl -a /dev/sda", desc: "disk health (package smartmontools)" }
paths:
  - { path: "/dev/disk/by-uuid/", desc: "stable names for disks and partitions" }
  - { path: "/sys/block/DEV/queue/", desc: "I/O scheduler, rotational flag" }
  - { path: "/etc/fstab", desc: "what gets mounted at boot" }
cards: [cmd.find]
---

Everything about storage devices: what's plugged in, how full it is, how it's
mounted.
