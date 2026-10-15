---
id: hw-cpu.frequency
kind: concept
title: Clock speed, turbo and power saving
aliases: [cpu frequency, clock speed, ghz, turbo, turbo boost, boost, governor, cpufreq, scaling governor, c-states, cpuidle, throttling, thermal throttling, cpu temperature, powersave]
hubs: [cpu]
probes: [cpu.freq]
see_also: [hw-cpu.cores, cpu.load-average]
commands: [cpupower, sensors, grep]
paths: [/sys/devices/system/cpu/cpu0/cpufreq/, /sys/class/thermal/]
---

## TL;DR

- A CPU's speed changes all the time: it climbs when there's work, drops
  when idle, and goes above its base frequency (turbo) while power and
  heat allow.
- The kernel's cpufreq driver and governor pick the speed; idle CPUs drop
  into sleep states (C-states), deeper ones saving more but waking slower.
- Too hot, and the CPU slows itself down (thermal throttling): a laptop
  can lose half its speed under a long load.

```
grep MHz /proc/cpuinfo                # current speed of each CPU (x86)
cpupower frequency-info               # driver, governor, limits
sensors                               # temperatures (package lm_sensors)
```

## Pitfalls

- With `intel_pstate` and `amd-pstate`, the `powersave` governor still
  reaches full speed under load: it isn't "always slow".
- Benchmarks drift as the chip heats and turbo fades: warm up first, and
  compare runs on the same power profile, plugged in.
- A virtual machine often sees one fixed, made-up frequency: the host
  decides the real one.
- A slow machine with a low load may be throttling: check temperatures
  and the fan before blaming software.

## Deep dive

### Why not always run at full speed?

Power grows much faster than frequency: a higher clock needs a higher
voltage, and power rises with the square of the voltage. Running a burst
fast and then sleeping deeply ("race to idle") often costs less energy
than running slowly for longer. Turbo exploits the other side: the chip
is allowed to exceed its sustained power limit for a short while, until
heat builds up.

### What are C-states?

C0 is running. In C1 the core stops its clock but wakes almost
instantly; deeper states turn off caches and power, and take tens or
hundreds of microseconds to wake. The cpuidle governor guesses how long
the CPU will stay idle and picks the deepest state worth it. For very
latency-sensitive work, limiting deep states trades power for
reactivity.
