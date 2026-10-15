---
id: logs.journalctl
kind: concept
title: Reading the journal with journalctl
aliases: [journalctl, journalctl -f, journalctl -b, journalctl -p, journald, systemd journal, log priority, follow logs, previous boot logs, journalctl -g]
hubs: [logs]
probes: [log.journal]
see_also: [logs.where, systemd.failing, kernel.dmesg]
commands: [journalctl]
paths: [/var/log/journal/]
---

## TL;DR

- systemd's journal collects everything: the kernel, every service's
  output, and programs that log through syslog. One command reads it.
- Narrow it down: by boot (`-b`), unit (`-u`), priority (`-p`), time
  (`--since`), text (`-g`). `-f` follows new lines as they come.
- Reading other users' and system logs needs `sudo`, or membership in
  `adm` or `systemd-journal`.

```
journalctl -b -p warning              # warnings and worse, this boot
journalctl -u nginx --since "1 hour ago"
journalctl -f -u myapp                # follow one service
journalctl -b -1 -e                   # end of the previous boot
```

## Pitfalls

- `-b -1` shows nothing: the journal is kept in RAM only, so earlier
  boots are gone ({{logs.where}}).
- Priorities count down: `-p err` means error and worse (crit, alert,
  emerg), not just errors.
- `--since` takes local time; `--utc` shows and parses times in UTC.
- Output goes through a pager: `--no-pager` for scripts, `-o json` to
  parse it.

## Deep dive

### What is stored besides the message?

Each entry carries fields: `_SYSTEMD_UNIT`, `_PID`, `_UID`, `_COMM`,
`PRIORITY`, the boot ID and more, set by journald itself so programs
can't fake them. Any field filters: `journalctl _UID=1000`,
`journalctl _COMM=sshd`. `journalctl -o verbose` shows them all.

### Why a binary format?

Indexed by field and time, so filtering a month of logs by unit is
instant, and entries can be checked for tampering (`--verify`). The
cost: you need `journalctl` to read them, not `grep` on a file.
