---
id: bin.elf
kind: concept
title: ELF and "command not found"
aliases: [command not found, elf, no such file or directory, exec format error, bad interpreter, shebang, cannot execute binary file, file command]
hubs: []
probes: [bin.path]
see_also: [bin.path, bin.ldd, perms.rwx]
commands: [file, readelf]
paths: []
---

## TL;DR

- "command not found": no file of that name in any folder of your PATH.
  Install the package, or call the program by its full path.
- "No such file or directory" for a file that exists: what's missing is its
  interpreter, the `#!` line of a script, or the loader a binary was built
  for.
- "Exec format error": the binary is built for another CPU (ARM vs x86).

```
file ./mytool          # architecture, linking, interpreter
head -1 deploy.sh      # the #! line of a script
```

## Pitfalls

- A script saved with Windows line endings asks for `/bin/bash\r`: "bad
  interpreter". Fix it with `dos2unix` or `sed -i 's/\r$//' deploy.sh`.
- "Permission denied" on your own file: it lacks the `x` bit (`chmod +x`),
  or sits on a filesystem mounted `noexec`.
- Programs built for musl (Alpine) don't run on glibc systems, nor the
  other way round, unless they are statically linked.
- To find the package that provides a command: `apt-file search bin/NAME`
  (Debian, Ubuntu), `dnf provides '*/bin/NAME'` (Fedora), `pacman -F NAME`
  (Arch).

## Deep dive

### What is ELF?

The file format of Linux programs and libraries. Its header names the CPU
architecture; then come the code and the data. A dynamically linked
program also names its interpreter, the loader that the kernel starts first
to load the libraries: `/lib64/ld-linux-x86-64.so.2` for x86-64 with glibc.
`readelf -l program` shows it as "Requesting program interpreter".

### What does the kernel do with a script?

When a file starts with `#!`, the kernel runs the program named on that
line and gives it the script. `#!/usr/bin/env python3` asks `env` to find
`python3` through PATH, so the same script works where Python is installed
in different places.
