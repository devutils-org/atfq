---
id: cmd.grep
kind: concept
title: grep and regex
aliases: [grep, regex, regular expression, regexp, egrep, rg, ripgrep, search text]
hubs: []
probes: []
see_also: [cmd.find, cmd.sed]
commands: [grep, rg]
paths: []
---

## TL;DR

- `grep` prints the lines that match a pattern, in files or in what you pipe
  into it.
- `-r` searches a whole folder, `-i` ignores case, `-n` shows line numbers,
  `-v` keeps the lines that don't match.
- `-E` for the usual regex syntax (`+`, `?`, `|`, `()`), `-F` to search a
  plain string.

```
grep -rn 'TODO' src/
journalctl -b | grep -i 'error'
grep -E 'nginx|php-fpm' services.txt
```

## Pitfalls

- Put the pattern in single quotes, or the shell may change `*`, `$` or `!`
  before grep sees it.
- Without `-E`, `|` and `+` match themselves: `grep 'a|b'` looks for the
  three characters `a|b`.
- `.` matches any character: `grep '10.0.0.1'` also matches `10a0b0c1`. Use
  `-F` for a literal string.
- `ps aux | grep nginx` also lists the grep itself: `pgrep -a nginx` doesn't.
- `grep -r` reads binary files too: add `-I` to skip them. It doesn't follow
  symbolic links; `-R` does.

## Deep dive

### What does a regex look like?

`^` is the start of the line and `$` its end. `.` is any character, `[abc]`
one of these, `[^0-9]` anything but a digit. `*` repeats what comes before
zero or more times. With `-E`: `+` once or more, `?` optional, `{2,4}`
between 2 and 4 times, `a|b` either one, and `( )` groups.

```
grep -E '^[0-9]{3}-[0-9]{4}$' numbers.txt
```

### grep, egrep or ripgrep?

`egrep` and `fgrep` are old names for `grep -E` and `grep -F`; since version
3.8, GNU grep warns when you use them. `rg` (ripgrep, package `ripgrep`) is
a faster alternative: it searches recursively by default and skips hidden
files, binary files and anything in `.gitignore`.
