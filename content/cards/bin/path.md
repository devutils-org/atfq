---
id: bin.path
kind: concept
title: PATH and command resolution
aliases: [path, $path, which, command -v, type, hash -r, command resolution, .local/bin]
hubs: []
probes: [bin.path]
see_also: [bin.elf, shell.startup]
commands: [type, which]
paths: [/usr/local/bin, ~/.local/bin]
---

## TL;DR

- PATH is a list of folders separated by `:`. The shell looks for a command
  in each, left to right, and runs the first match.
- `type NAME` says what a name is: alias, function, builtin or file.
  `type -a` shows every match.
- Your own scripts usually go in `~/.local/bin`. Many distros add it to
  PATH when it exists; otherwise add it in a startup file ({{shell.startup}}).

```
echo "$PATH" | tr : '\n'
type -a python3
export PATH="$HOME/.local/bin:$PATH"
```

## Pitfalls

- bash remembers where it found each command: after installing or moving
  one, `hash -r` makes it search again.
- An empty entry or `.` in PATH means "the current folder": a file named
  `ls` planted there would run instead of the real one.
- `which` is a separate program that doesn't know your aliases and
  functions; `type` tells what your shell will actually run.
- `sudo` uses its own PATH (`secure_path` in sudoers): a command your shell
  finds can be "not found" under sudo.

## Deep dive

### In what order does the shell look?

For a name without a `/`: aliases first, then functions, then builtins
(`cd`, `echo`), then the PATH folders. A name with a `/` in it, like
`./build.sh` or `/usr/bin/python3`, is a path: no search at all.

### Why /usr/local/bin before /usr/bin?

So that a program you install by hand wins over the packaged program of the
same name, without touching the files the package manager owns. The same
goes for `~/.local/bin` placed first: your version wins, for you only.
