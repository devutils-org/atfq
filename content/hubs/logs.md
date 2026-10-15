---
id: logs
kind: hub
title: Logs
aliases: [log, logging, journal, system logs]
probes: [log.journal]
commands:
  - { cmd: "journalctl -b -p warning", desc: "warnings and errors since boot", card: logs.journalctl }
  - { cmd: "journalctl -u nginx -f", desc: "follow one service's log", card: logs.journalctl }
  - { cmd: "journalctl -b -1 -e", desc: "how the previous boot ended", card: logs.where }
  - { cmd: "journalctl --disk-usage", desc: "space the journal takes", card: logs.rotation }
  - { cmd: "sudo dmesg -w", desc: "kernel messages as they arrive", card: kernel.dmesg }
  - { cmd: "ls -lt /var/log | head", desc: "text logs written most recently", card: logs.where }
paths:
  - { path: "/var/log/journal/", desc: "the journal, when it's kept on disk" }
  - { path: "/var/log/", desc: "text logs from rsyslog and programs" }
  - { path: "/etc/systemd/journald.conf", desc: "journal storage and size limits" }
  - { path: "/etc/logrotate.d/", desc: "rotation rules for text logs" }
cards: [logs.journalctl, logs.where, logs.rotation, logs.app, kernel.dmesg]
---

What happened on this machine, where it's written down, and how to keep
it from filling the disk.
