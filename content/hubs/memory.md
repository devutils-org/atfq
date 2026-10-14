---
id: memory
kind: hub
title: Memory
aliases: [ram, mem]
probes: [mem.summary]
commands:
  - { cmd: "free -h", desc: "RAM and swap, in human units" }
  - { cmd: "vmstat 1", desc: "memory and swap activity every second (Ctrl-C stops)" }
  - { cmd: "top -o %MEM", desc: "processes sorted by memory use" }
  - { cmd: "swapon --show", desc: "swap areas and how full they are" }
  - { cmd: "sudo dmesg | grep -i oom", desc: "recent work of the OOM killer" }
paths:
  - { path: "/proc/meminfo", desc: "the kernel's memory counters, read by free and top" }
  - { path: "/proc/sys/vm/swappiness", desc: "how willing the kernel is to swap" }
  - { path: "/proc/PID/status", desc: "memory of one process (VmRSS line)" }
cards: []
---

How much RAM you have, who is using it, and why "used" doesn't mean what you
think.
