---
id: text.locales
kind: concept
title: "Locales: why sorting, dates and numbers change"
aliases: [locales, lc_all, lc_ctype, lc_collate, lc_numeric, lc_time, locale-gen, localectl, collation, setlocale, cannot change locale, c.utf-8, posix locale]
hubs: []
probes: [text.locale]
see_also: [text.encodings, shell.env, cmd.sort-uniq-cut]
commands: [locale, localectl, sort]
paths: [/etc/locale.conf, /etc/default/locale, /etc/locale.gen]
---

## TL;DR

- A locale bundles cultural rules: how bytes become characters
  (LC_CTYPE), sort order (LC_COLLATE), number and date formats, message
  language. Set by `LANG`, overridden by `LC_*`, all overridden by
  `LC_ALL`.
- The same command can give different results in another locale: `sort`
  order, `3,14` vs `3.14`, month names.
- In scripts, pin it: `LC_ALL=C` for byte order and plain ASCII rules, or
  `C.UTF-8` to keep UTF-8.

```
locale                       # current settings
locale -a                    # locales installed here
LC_ALL=C sort file.txt       # sort by bytes, the same everywhere
```

## Pitfalls

- `printf '%.2f' 3.14` fails in a French locale ("invalid number"):
  scripts that parse or print numbers should set `LC_NUMERIC=C`.
- "setlocale: cannot change locale" (often over SSH): your terminal sends
  a `LANG` the server hasn't generated. Generate it there (`locale-gen`)
  or set `LANG=C.UTF-8`.
- `[a-z]` can match accented or uppercase letters, depending on the
  locale and the tool's version; `[[:lower:]]` or `LC_ALL=C` say what you
  mean.

## Deep dive

### Why does sort order depend on the locale?

Human sorting rules differ: ignoring case and accents at first, then
breaking ties; Swedish puts `ä` after `z`, German next to `a`. The C
locale skips all that and compares bytes, which is fast and the same
everywhere, but puts `Zebra` before `apple`. Two programs sorting in
different locales can disagree, which breaks tools like `join` and
`comm` that expect sorted input.

### Where do locales come from?

Locale definitions are compiled into binary files, one per locale or
all in `/usr/lib/locale/locale-archive`. Debian and Arch compile only
those listed in `/etc/locale.gen`; Fedora ships them in language packs.
`C.UTF-8` is built into recent glibc and always available.
