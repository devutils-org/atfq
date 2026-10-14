---
id: bin.ldd
kind: concept
title: Dynamic linking and ldd
aliases: [ldd, shared library, shared libraries, .so, ld.so, ldconfig, ld_library_path, error while loading shared libraries, dynamic linking]
hubs: []
probes: [bin.path]
see_also: [bin.elf, kernel.strace]
commands: [ldd, ldconfig, objdump]
paths: [/etc/ld.so.conf.d/]
---

## TL;DR

- Most programs don't contain the libraries they use: at start-up, the
  dynamic loader finds the `.so` files they need and loads them.
- `ldd program` lists those libraries and where each was found. "not found"
  marks the one that's missing.
- The loader searches `LD_LIBRARY_PATH` first, then the folders `ldconfig`
  knows (from `/etc/ld.so.conf.d/`), then the standard ones.

```
ldd /usr/bin/curl
ldconfig -p | grep libssl
```

## Pitfalls

- "error while loading shared libraries: libfoo.so.1": install the package
  that provides it, rather than copying or linking another version.
- After putting a library in a new folder, list that folder in a file in
  `/etc/ld.so.conf.d/`, then run `sudo ldconfig`.
- Don't run `ldd` on a program you don't trust: it may run the program's
  code. `objdump -p program | grep NEEDED` only reads it.
- `LD_LIBRARY_PATH` set for everything breaks other programs: set it only
  for the command that needs it.

## Deep dive

### Why link dynamically?

One copy of a library serves every program, on disk and in RAM, and a
security fix in the library fixes all of them at once. Static linking puts
everything in the program: easier to copy to another machine, but bigger,
and every program must be rebuilt to get a fix. Go programs are often
linked that way.

### What is the number in libfoo.so.1?

The major version, part of the library's "soname". A program asks for
`libfoo.so.1`; the file is often `libfoo.so.1.2.3`, and `libfoo.so.1` is a
link to it. A new major version (`.so.2`) means an incompatible change, and
it can be installed next to the old one.
