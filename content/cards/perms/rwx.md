---
id: perms.rwx
kind: concept
title: rwx, octal and umask
aliases: [permissions, file permissions, chmod, umask, rwx, octal, permission denied]
hubs: []
probes: [user.identity]
see_also: [perms.special, perms.sudo-groups]
commands: [chmod, chown, umask]
paths: []
---

## TL;DR

- Every file has an owner, a group, and three sets of rights: for the
  owner, for the group, for everyone else.
- `r` read, `w` write, `x` execute. On a folder, `x` means "may enter".
- In octal, r=4, w=2, x=1, added per set: `644` is `rw-r--r--`, `755` is
  `rwxr-xr-x`.
- The umask removes rights from new files: with `022`, files get 644 and
  folders 755.

```
ls -l notes.txt
chmod 640 notes.txt     # owner rw, group r, others nothing
chmod u+x deploy.sh     # let the owner run it
```

## Pitfalls

- On a folder, `r` lists the names and `x` lets you open what's inside:
  with `r` but no `x`, you see names you can't open.
- Deleting a file depends on the folder's `w`, not on the file's rights.
- `chmod -R 777` "fixes" an error by letting everyone write everything. Fix
  the owner or the group instead.
- Root ignores read and write rights, but still needs some `x` bit to run a
  file.
- The umask never adds rights: new files start from 666, folders from 777,
  so a new file is never executable.

## Deep dive

### Why octal?

Each set of rights is three bits, and one octal digit is exactly three
bits, so `750` reads as owner 7 (rwx), group 5 (r-x), others 0. A fourth
digit in front holds the setuid, setgid and sticky bits (see {{perms.special}}):
`chmod 2775 shared/`.

### Where does the umask come from?

A process inherits it from its parent. Your login sets it, from
`/etc/login.defs` or a shell startup file, and you can change it for the
current shell with `umask 077`. systemd services use the `UMask=` setting
of their unit, 0022 by default.

Common values are 022 (others can read) and 077 (private). Some distros use
002 for regular users, since each user has a private group of their own.
