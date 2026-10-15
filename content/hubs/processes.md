---
id: processes
kind: hub
title: Processes & scheduling
aliases: [process, running programs, tasks]
probes: [proc.states, proc.top]
commands:
  - { cmd: "ps aux --sort=-%mem | head", desc: "processes using the most memory" }
  - { cmd: "top", desc: "live view; press M for memory, P for CPU" }
  - { cmd: "pgrep -a nginx", desc: "find processes by name, with their command", card: proc.signals }
  - { cmd: "pstree -p", desc: "who started whom, with PIDs" }
  - { cmd: "kill 1234", desc: "ask a process to stop", card: proc.signals }
  - { cmd: "systemd-cgls", desc: "processes grouped by service and session", card: proc.cgroups }
paths:
  - { path: "/proc/PID/", desc: "everything about one process" }
  - { path: "/proc/PID/status", desc: "state, memory, threads, owner" }
cards: [internals.exec, internals.scheduler, cpu.load-average, memory.oom-killer]
---

What's running, who started it, what it costs, and how to stop it.
