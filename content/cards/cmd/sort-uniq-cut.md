---
id: cmd.sort-uniq-cut
kind: concept
title: sort, uniq and cut
aliases: [sort, uniq, cut, wc, count lines, duplicates, unique lines]
hubs: []
probes: []
see_also: [cmd.awk]
commands: [sort, uniq, cut, wc]
paths: []
---

## TL;DR

- `sort` sorts lines: `-n` by number, `-h` by size with units (2K, 1G), `-r`
  in reverse, `-k 2` by the second column.
- `uniq` removes repeated lines that follow each other, so sort first.
  `uniq -c` counts them.
- `cut` keeps some columns: `-d,` sets the separator, `-f 1,3` the columns.

```
sort ips.txt | uniq -c | sort -rn | head    # most frequent lines
du -sh * | sort -h                          # biggest items last
cut -d: -f1,7 /etc/passwd                   # users and their shells
```

## Pitfalls

- `uniq` without `sort` only merges neighbours: an unsorted file keeps its
  duplicates.
- `sort` without `-n` compares text, so 10 comes before 9.
- `cut` splits on one character: with columns separated by several spaces,
  use `awk` (see 6.3).
- The order depends on your language settings (case, accents).
  `LC_ALL=C sort` sorts by byte value, and faster.

## Deep dive

### Why so many small tools?

That's the Unix approach: each tool does one thing to lines of text, and
pipes connect them. `sort | uniq -c | sort -rn | head` is a "most frequent
items" program in one line, built from parts you already know.

### Shortcuts worth knowing

`sort -u` sorts and drops duplicates in one step. `uniq -d` keeps only the
lines that were repeated. `wc -l` counts lines, and `sort -t, -k3,3n` sorts
a comma-separated file numerically by its third column.
