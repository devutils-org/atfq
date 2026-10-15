---
id: cpu
kind: hub
title: CPU
aliases: [processor, cpus, cores]
probes: [cpu.summary]
commands:
  - { cmd: "nproc", desc: "number of CPUs you can use" }
  - { cmd: "lscpu", desc: "model, cores, threads and caches" }
  - { cmd: "uptime", desc: "load average over 1, 5 and 15 minutes", card: cpu.load-average }
  - { cmd: "top", desc: "busiest processes; press 1 for one line per CPU" }
  - { cmd: "mpstat -P ALL 1", desc: "usage of each CPU every second (package sysstat)" }
paths:
  - { path: "/proc/cpuinfo", desc: "one block per logical CPU" }
  - { path: "/proc/loadavg", desc: "load average and task counts" }
  - { path: "/sys/devices/system/cpu/", desc: "online CPUs, frequency, governor" }
cards: [internals.scheduler, internals.interrupts, proc.nice]
---

What your processor is, how busy it is, and who keeps it busy.
