---
id: shell.env
kind: concept
title: Where environment variables come from
aliases: [env, environment, environment variables, env vars, export, printenv, /etc/environment, locale, lang]
hubs: []
probes: [shell.env]
see_also: [shell.startup, bin.path]
commands: [env, printenv, export]
paths: [/etc/environment, ~/.config/environment.d/]
---

## TL;DR

- Every process gets a copy of its parent's environment: a list of
  `NAME=value` strings.
- `export NAME=value` sets it for the current shell and the programs it
  starts, not for its parent or for other terminals.
- To keep it, put the `export` in a startup file ({{shell.startup}}). For graphical apps
  on systemd desktops, use a file in `~/.config/environment.d/`.

```
printenv LANG
export EDITOR=nvim
LANG=C sort names.txt    # set for one command only
```

## Pitfalls

- A variable set without `export` stays in the shell: the programs it
  starts don't see it.
- A change never reaches programs already running, nor other terminals:
  open a new one, or `source ~/.bashrc`.
- systemd services don't read your shell files: give them variables with
  `Environment=` in a drop-in (see {{systemd.overrides}}).
- `sudo` resets most of the environment. `sudo -E` keeps it, when sudoers
  allows it.
- Variables are visible to every process you run, in
  `/proc/PID/environ`: avoid keeping secrets there for long.

## Deep dive

### Where does the first environment come from?

At login, PAM and the login program set `HOME`, `USER`, `SHELL` and a
default `PATH`; `pam_env` adds what's in `/etc/environment`. The login
shell then runs its startup files, which add more. Desktop sessions started
by systemd also read `environment.d` files. Every program after that
inherits the result.

### What do LANG and LC_ALL do?

They choose the language of messages, the format of numbers and dates, the
sort order and the character encoding. `LC_ALL` overrides everything,
`LANG` is the fallback. `locale` shows the current values, and `LC_ALL=C`
gives scripts stable, byte-based behaviour.
