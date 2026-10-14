---
id: proc.signals
kind: concept
title: Signals
aliases: [signals, signal, kill, pkill, sigterm, sigkill, sighup, sigint]
hubs: []
probes: []
see_also: [proc.states]
commands: [kill, pkill, pgrep]
paths: []
---

## TL;DR

- A signal is a short message the kernel hands to a process: stop, pause,
  continue, reload.
- `kill` sends SIGTERM by default: "please stop". The process can clean up
  first.
- SIGKILL (`kill -9`) can't be caught or ignored: the kernel ends the
  process at once.
- Ctrl-C sends SIGINT, Ctrl-Z pauses the program (SIGTSTP), `fg` resumes it.

```
kill 1234          # polite: SIGTERM
kill -9 1234       # last resort: SIGKILL
pgrep -af backup   # find processes by command line
```

## Pitfalls

- Start with a plain `kill`: `-9` skips cleanup, and temp files, locks and
  half-written data stay behind.
- `kill -9` does nothing to a zombie, and a process stuck in D state only
  dies when its I/O ends (see 4.1).
- `pkill -f` matches anywhere in the command line: check what it would hit
  with `pgrep -af` first.
- Closing a terminal sends SIGHUP to what runs in it. For long tasks, use
  `nohup` or a tmux session.
- Many daemons reload their settings on SIGHUP, but not all: prefer
  `systemctl reload` when the service supports it.

## Deep dive

### What does a process do with a signal?

For each signal, a process keeps the default action (often: exit), ignores
it, or runs a handler of its own. SIGKILL and SIGSTOP are the two it can't
catch or ignore.

That's how a well-written server stops cleanly: it catches SIGTERM, stops
accepting work, finishes what's in progress, then exits.

### How do systemd and Docker stop things?

`systemctl stop` sends SIGTERM, waits (90 seconds by default, set by
`TimeoutStopSec=`), then sends SIGKILL. `docker stop` does the same with a
10 second wait. A service that takes long to stop is often one that ignores
SIGTERM and waits for the SIGKILL.
