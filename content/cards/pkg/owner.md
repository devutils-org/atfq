---
id: pkg.owner
kind: concept
title: Which package owns a file
aliases: [package, packages, which package, package manager, dpkg -s, rpm -qf, pacman -qo, owns]
hubs: []
probes: [pkg.manager]
see_also: [pkg.formats, bin.elf]
commands: [dpkg, rpm, pacman]
paths: []
---

## TL;DR

- Every file a package installs is recorded in the package database: ask
  it who owns a file.
- `dpkg -S` on Debian and Ubuntu, `rpm -qf` on Fedora and openSUSE,
  `pacman -Qo` on Arch.
- For a program that isn't installed yet, search the repositories instead
  (see 12.2).

```
dpkg -S /usr/bin/curl
rpm -qf /usr/bin/curl
pacman -Qo /usr/bin/curl
```

## Pitfalls

- Give the full path: `dpkg -S curl` matches every path that contains
  "curl". `command -v curl` gives the path.
- Files you created, and settings a package generated after install,
  belong to no package.
- With merged /usr (see 1.1), ask about `/usr/bin/…` rather than `/bin/…`:
  the database may only know one of the two.

## Deep dive

### What's in the package database?

For each package: its version, its dependencies, and the list of its files
with checksums. That's how `dpkg --verify`, `rpm -V` and `pacman -Qkk` can
tell which installed files were changed since installation.

### Why does it matter?

Before editing or deleting a file under `/usr` or `/etc`, knowing its
package tells you whether an update will overwrite your change, or bring a
deleted file back. Changes belong in your own files: a drop-in, a file in
`/etc`, or `/usr/local`.
