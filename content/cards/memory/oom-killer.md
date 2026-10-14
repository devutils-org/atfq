---
id: memory.oom-killer
kind: concept
title: The OOM killer
aliases: [oom, oom killer, out of memory, oom_score, systemd-oomd, killed process]
hubs: [memory]
probes: [mem.summary]
see_also: [memory.swap, proc.signals]
commands: [journalctl, dmesg]
paths: [/proc/PID/oom_score, /proc/PID/oom_score_adj]
---

## TL;DR

- When RAM and swap are both used up, the kernel's OOM ("out of memory")
  killer picks a process and kills it to get memory back.
- It mostly picks the biggest user of memory, adjusted by each process's
  `oom_score_adj` (-1000 to 1000).
- The victim gets SIGKILL: no warning, no cleanup. The kernel log says who.

```
journalctl -k | grep -i "out of memory"
cat /proc/1234/oom_score
```

## Pitfalls

- On Fedora and Ubuntu, systemd-oomd can kill a whole app earlier, when
  memory pressure stays high: check `journalctl -u systemd-oomd`.
- In a container or a service with a memory limit, the OOM killer acts
  inside that limit, even if the machine has free RAM.
- A machine that swaps hard can stay frozen for minutes before the OOM
  killer acts.
- `oom_score_adj = -1000` makes a process immune. Use it sparingly, or
  something else important gets killed instead.
- Reading the kernel log may need `sudo` or membership of the `adm` or
  `systemd-journal` group.

## Deep dive

### Why not refuse to give out memory instead?

Linux overcommits: it promises more memory than it has, because programs
often reserve far more than they ever touch. Memory is only taken when a
page is first written. When too many promises come due at once, there is no
request left to refuse, so the kernel has to take memory back.

`vm.overcommit_memory` changes the policy: 0 (default) guesses, 1 always
says yes, 2 never promises more than a set limit.

### How is the victim chosen?

Each process gets a score, visible in `/proc/PID/oom_score`: roughly its
share of RAM and swap in use, in thousandths, plus its `oom_score_adj`. The
highest score is killed. A systemd unit can set it with `OOMScoreAdjust=`,
so a critical daemon is less likely to be picked.
