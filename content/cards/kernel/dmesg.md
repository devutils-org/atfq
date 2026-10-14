---
id: kernel.dmesg
kind: concept
title: dmesg and the kernel log
aliases: [dmesg, kernel log, kmsg, kernel messages, dmesg_restrict]
hubs: []
probes: [kernel.sysctl]
see_also: [kernel.modules, memory.oom-killer]
commands: [dmesg, journalctl]
paths: [/dev/kmsg]
---

## TL;DR

- The kernel keeps its messages in a buffer in RAM: hardware found, driver
  errors, OOM kills, a USB stick plugged in.
- `dmesg` prints that buffer. `journalctl -k` shows the same messages, kept
  across reboots when the journal is persistent.
- With `kernel.dmesg_restrict = 1`, the default on many distros, reading it
  needs root.

```
sudo dmesg -T | tail -20       # with human-readable times
sudo dmesg -w                  # follow new messages
journalctl -k -b -1 -p err     # errors from the previous boot
```

## Pitfalls

- The buffer has a fixed size: on a busy machine, early boot messages are
  overwritten. `journalctl -k -b` still has them.
- `dmesg -T` times can be wrong after a suspend: the kernel's clock doesn't
  count the time spent suspended.
- To see how the kernel names a device, run `sudo dmesg -w`, then plug it in.
- A "Call Trace" block means the kernel hit a bug or a warning: the whole
  block, not just one line, is what a bug report needs.

## Deep dive

### Why does dmesg need root?

Kernel messages can contain memory addresses, which help an attacker get
around protections such as address randomization. So many distros set
`kernel.dmesg_restrict = 1`, and `dmesg` then needs root. Members of the
`adm` or `systemd-journal` group can still read the same messages with
`journalctl -k`.

### What are log levels?

Each kernel message has a level, from 0 (emergency) to 7 (debug); 3 is an
error, 4 a warning. `dmesg -l err,warn` shows only those, and
`journalctl -k -p warning` does the same from the journal.
