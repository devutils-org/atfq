---
id: time.clocks
kind: concept
title: "Wall clock and monotonic clock: measuring time right"
aliases: [wall clock, monotonic clock, clock_monotonic, clock_realtime, clock_gettime, system time, clocksource, tsc, rtc, hardware clock, hwclock, time measurement]
hubs: []
probes: [time.clock]
see_also: [time.ntp, internals.interrupts]
commands: [date, timedatectl]
paths: [/sys/devices/system/clocksource/]
---

## TL;DR

- The wall clock says what time it is. It can jump: NTP corrects it, an
  admin sets it, a VM resumes. Never measure durations with it.
- The monotonic clock only moves forward, from an arbitrary start (boot):
  made for durations and timeouts.
- A battery-powered hardware clock (RTC) keeps time while the machine is
  off; the kernel reads it at boot, then counts with a CPU counter.

```
date                       # wall clock, in your time zone
cat /proc/uptime           # seconds since boot: monotonic
timedatectl                # wall clock, RTC, zone, NTP status
```

## Pitfalls

- `end - start` with wall-clock times can be negative or huge across an
  NTP step: use `time.monotonic()` (Python), `performance.now()` (JS),
  `Instant` (Rust); Go's `time.Since` already does.
- The monotonic clock stops during suspend; `CLOCK_BOOTTIME` doesn't.
- Comparing timestamps from two machines assumes both clocks are right:
  their difference is only as good as NTP ({{time.ntp}}).

## Deep dive

### Where does the kernel get the time from?

A clock source: a counter ticking at a known, steady rate. On x86 it's
the TSC, a CPU register counting cycles; on ARM, the architected timer
(`arch_sys_counter`); in VMs, often a paravirtual clock (`kvm-clock`).
Reading it takes nanoseconds and needs no system call: `clock_gettime`
runs in user space through the vDSO.

### Why can the wall clock go backwards?

Because it's an estimate of real time, corrected from outside. NTP
usually adjusts it gradually, speeding it up or slowing it slightly
(slewing), but steps it when it's far off, at boot or after a long
suspend. The monotonic clock is slewed too, but never stepped.
