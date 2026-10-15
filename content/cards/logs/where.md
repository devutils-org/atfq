---
id: logs.where
kind: concept
title: "Where logs go: journald, syslog and /var/log"
aliases: [/var/log, var/log, rsyslog, syslog-ng, syslog, journald.conf, persistent journal, volatile journal, auth.log, /var/log/syslog, /var/log/messages, log files]
hubs: [logs]
probes: [log.journal]
see_also: [logs.journalctl, logs.rotation]
commands: [journalctl, tail, less]
paths: [/var/log/, /etc/systemd/journald.conf, /etc/rsyslog.conf]
---

## TL;DR

- journald receives the logs. It keeps them on disk in
  `/var/log/journal/` if that folder exists, otherwise in RAM under
  `/run/log/journal/`, lost at reboot.
- A syslog daemon (rsyslog) may also copy them to text files:
  `/var/log/syslog` (Debian, Ubuntu) or `/var/log/messages` (older
  RHEL), and `auth.log` for logins.
- Some programs write their own files: `/var/log/nginx/`,
  `/var/log/apt/`, `/var/log/pacman.log`.

```
sudo mkdir -p /var/log/journal && sudo systemctl restart systemd-journald
ls -lt /var/log | head
sudo tail -f /var/log/nginx/error.log
```

## Pitfalls

- Logs from before a crash need a persistent journal: create
  `/var/log/journal` (or set `Storage=persistent`) before you need them.
- In containers there's often no journald: logs go to the container's
  output (`docker logs`).
- `/var/log/wtmp` and `btmp` are binary login records: read them with
  `last` and `lastb`.

## Deep dive

### Why both journald and rsyslog?

journald arrived with systemd (2011); rsyslog was already everywhere,
with remote logging, text files that tools expected, and filtering rules
admins had written. Many distros kept both, journald forwarding to
rsyslog. Recent ones increasingly drop rsyslog and rely on the journal
alone, which can send logs to another machine itself.

### How do logs reach journald?

Three ways: a service's standard output and error, captured by systemd;
the syslog socket (`/dev/log`) that the `syslog()` call writes to; and
journald's own API with structured fields. The kernel's messages are read
from `/dev/kmsg`.
