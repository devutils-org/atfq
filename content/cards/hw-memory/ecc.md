---
id: hw-memory.ecc
kind: concept
title: ECC and failing RAM
aliases: [ecc, ecc memory, ecc ram, edac, memory errors, memory error, bad ram, faulty ram, memtest, memtest86, memtest86+, rasdaemon, bit flip, machine check, mce]
hubs: [memory]
probes: [mem.errors]
see_also: [hw-memory.hierarchy, kernel.dmesg]
commands: [ras-mc-ctl, journalctl]
paths: [/sys/devices/system/edac/mc/]
---

## TL;DR

- RAM can return a wrong bit, mostly from a failing or badly seated
  module. Without ECC nothing notices: random crashes, corrupted files.
- ECC memory stores extra check bits: it corrects a single wrong bit and
  detects two, and the kernel counts both (EDAC).
- Corrected errors are a warning; a count that keeps growing on one module
  means replacing it. An uncorrectable error stops the program, or the
  machine.

```
sudo ras-mc-ctl --error-count       # per module (package rasdaemon)
journalctl -k | grep -i -E 'edac|mce|hardware error'
```

## Pitfalls

- Random segfaults in different programs, or a compiler crashing on code
  that built yesterday, point at RAM: test it before reinstalling.
- Test with memtest86+ from a USB stick or a boot menu entry, for several
  passes. Testing from the running system can't reach every address.
- ECC needs a CPU, board and modules that all support it: ECC modules
  in a desktop board often run without it.
- No EDAC entries doesn't prove good RAM: it may only mean there's no ECC.

## Deep dive

### How does ECC correct a bit?

For every 64 bits of data, an ECC module stores 8 more: a code computed
from the data (SECDED: single error correct, double error detect). On
each read, the controller recomputes it. A single flipped bit gives a
pattern that points at the bit, which is fixed on the fly; two flipped
bits are detected but can't be located.

### What happens on an uncorrectable error?

The CPU raises a machine check. If the bad memory belongs to a process,
the kernel can kill just that process and retire the page, so it's never
used again (`HardwareCorrupted` in `/proc/meminfo`); if the kernel itself
was using it, the machine panics rather than carry on with wrong data.
