---
id: hw-cpu.speculation
kind: concept
title: Speculative execution and CPU flaws
aliases: [spectre, meltdown, speculative execution, cpu vulnerabilities, cpu vulnerability, mitigations, mitigations=off, branch prediction, microcode, side channel]
hubs: [cpu]
probes: [cpu.vulns]
see_also: [hw-cpu.instructions, boot.cmdline]
commands: [grep, journalctl]
paths: [/sys/devices/system/cpu/vulnerabilities/]
---

## TL;DR

- To stay busy, a CPU guesses: which way a branch goes, what memory will
  hold, and runs ahead. Wrong guesses are thrown away.
- Thrown away, but not without a trace: the caches keep what was read.
  Spectre, Meltdown and their successors (2018 onwards) use that to read
  memory a program should never see.
- The kernel works around each flaw it knows (mitigations), helped by
  microcode updates. It lists them per flaw.

```
grep -r . /sys/devices/system/cpu/vulnerabilities/
journalctl -k | grep -i microcode    # microcode loaded at boot
```

## Pitfalls

- Keep the microcode package installed and updated (`intel-microcode`,
  `amd-ucode` or `amd64-microcode`): many fixes need it.
- `mitigations=off` on the kernel command line buys some speed and drops
  every protection: only for isolated machines running trusted code.
- "Vulnerable" for one entry isn't always an alarm: some protections are
  only applied to programs that ask for them (`spec_store_bypass` is often
  left to prctl or seccomp).
- Mitigations cost most on older CPUs and system-call-heavy work; newer
  CPUs fix several flaws in hardware.

## Deep dive

### How can a cancelled guess leak a secret?

Take `if (i < size) x = table[secret[i] * 64];`. Trained to predict "true",
the CPU runs the body for an `i` out of bounds, reads a byte it shouldn't,
and loads one line of `table` depending on it. The result is discarded,
but that line stays in the cache. Timing reads of `table` then shows
which line is fast: that reveals the byte. This is Spectre variant 1.

### Why did Meltdown need a kernel change?

On affected Intel CPUs, a user program could speculatively read kernel
memory, mapped in every process for speed. The fix (KPTI) removes most of
the kernel from the page tables while user code runs, at the cost of
extra work on each system call. CPUs from about 2019 on aren't affected,
and the kernel skips KPTI on them.
