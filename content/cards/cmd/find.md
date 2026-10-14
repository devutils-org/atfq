---
id: cmd.find
kind: concept
title: find
aliases: [find, find files, find file, search files, find by name]
hubs: [disk]
probes: []
see_also: [cmd.xargs, cmd.grep, fs.inodes-links]
commands: [find, xargs]
paths: []
---

## TL;DR

- `find` walks a folder and everything under it, and prints each path that
  matches your tests.
- Give the folder first, then the tests: `-name`, `-type`, `-size`, `-mtime`.
- Put patterns in quotes so your shell leaves them alone.

```
find . -name '*.log'                   # by name, here and below
find ~/Downloads -type f -size +100M   # files over 100 MiB
find /var/log -type f -mtime -1        # changed in the last 24 hours
find . -name '*.tmp' -delete           # only after checking the list
```

## Pitfalls

- Without quotes, `*.log` is expanded by your shell before `find` sees it,
  and you get wrong results or a "paths must precede expression" error.
- `-delete` has no undo. Run the command without it first and check the
  list. Put it last: `find . -delete -name '*.tmp'` deletes everything.
- `-size -1M` matches only empty files, because sizes are rounded up to the
  unit. Use `-size -1024k` instead.
- `-name` looks at the file name only. Use `-path` to match the whole path,
  and `-iname` to ignore case.
- Piping into `xargs` breaks on names with spaces. Use `-exec ... {} +`, or
  `-print0 | xargs -0`.
- Add `2>/dev/null` to hide "Permission denied" noise, knowing it hides real
  errors too.

## Deep dive

### How does find read its expression?

For each file, `find` evaluates your expression from left to right, like a
condition. Tests such as `-name` and `-type` are true or false. Actions such
as `-print`, `-delete` and `-exec` do something, and also return true or
false. Two items next to each other mean AND, and evaluation stops at the
first false one.

That's why order matters: `-delete` placed before `-name` runs on every
file. If you give no action at all, `find` adds `-print`. Use `-o` for OR,
with escaped parentheses:

```
find . \( -name '*.jpg' -o -name '*.png' \)
```

### Should I end `-exec` with `\;` or `+`?

`-exec cmd {} \;` runs `cmd` once per file. `-exec cmd {} +` packs as many
files as possible into each run, like `xargs`, which is much faster on many
files. With `+`, the `{}` must come last, right before the `+`.

```
find . -name '*.txt' -exec grep -l 'TODO' {} +
```

### What about locate and fd?

`locate` searches a database of file names instead of walking the disk, so
it answers instantly. The database is rebuilt periodically by `updatedb`, so
recent files may be missing, and it can't filter by size or date.

`fd` is a modern alternative with a shorter syntax: it skips hidden files and
anything in `.gitignore` by default. It's not installed by default; on
Debian and Ubuntu the package is `fd-find` and the command is `fdfind`.
