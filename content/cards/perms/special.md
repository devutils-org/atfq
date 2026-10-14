---
id: perms.special
kind: concept
title: setuid, setgid and sticky bit
aliases: [setuid, setgid, suid, sgid, sticky, sticky bit, special permissions]
hubs: []
probes: []
see_also: [perms.rwx, fs.tmp]
commands: [chmod, getcap]
paths: [/usr/bin/passwd, /tmp]
---

## TL;DR

- setuid (`s` in the owner's `x` spot): the program runs with its owner's
  rights, not yours. That's how `passwd` can update `/etc/shadow`.
- setgid on a program: it runs with the file's group. On a folder: new
  files get the folder's group.
- Sticky bit (`t` on a folder): only a file's owner can delete it, even
  where everyone may write. `/tmp` has it.

```
ls -ld /usr/bin/passwd /tmp
chmod g+s shared/       # new files inherit the folder's group
```

## Pitfalls

- A capital `S` or `T` means the bit is set but the `x` under it isn't:
  usually a mistake.
- Linux ignores setuid on scripts (shell, Python…): it only works on
  compiled programs.
- setuid root programs are what attackers look for. List them with
  `find / -perm -4000 -type f 2>/dev/null` and know why each is there.
- A filesystem mounted with `nosuid` (common for USB drives and `/tmp`)
  ignores these bits.

## Deep dive

### How does setuid work?

A process has a real user, who started it, and an effective user, whose
rights it uses. Running a setuid file sets the effective user to the file's
owner. Programs like `passwd` and `sudo` check who you really are, then use
their extra power for that one job.

That makes every setuid root program a door into root: one bug in it can be
enough. That's why there are few of them, and why they are small.

### What replaces setuid today?

Capabilities split root's power into pieces, such as `cap_net_bind_service`
to listen on ports under 1024. A program gets the one piece it needs instead
of all of root. List the programs that have some with
`getcap -r /usr/bin 2>/dev/null`.
