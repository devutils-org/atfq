---
id: perms.sudo-groups
kind: concept
title: sudo and groups
aliases: [sudo, sudoers, groups, group, wheel, usermod, su, visudo]
hubs: []
probes: [user.identity]
see_also: [perms.rwx]
commands: [sudo, id, usermod, visudo]
paths: [/etc/sudoers, /etc/sudoers.d/, /etc/group]
---

## TL;DR

- `sudo` runs one command as root, if `/etc/sudoers` (or a file in
  `/etc/sudoers.d/`) allows it.
- Usually one group gets that right: `sudo` on Debian and Ubuntu, `wheel`
  on Fedora. On Arch, you enable the `wheel` line yourself.
- Group changes apply at your next login, not in shells already open.

```
id                              # your user and groups
sudo -l                         # what sudo lets you run
sudo usermod -aG docker alice   # add alice to the docker group
```

## Pitfalls

- `usermod -G` without `-a` replaces all your extra groups. Always `-aG`.
- Edit sudoers with `visudo`: it checks the syntax, and a broken sudoers
  can lock everyone out of sudo.
- Being in the `docker` group is as good as being root: whoever can talk to
  the Docker daemon can mount the whole disk into a container.
- `sudo echo x > /etc/file` fails: the redirection runs in your shell, not
  as root. Use `echo x | sudo tee /etc/file`.
- To use a new group without logging out, run `newgrp docker` in that
  shell.

## Deep dive

### Why groups and not lists of users?

Groups share rights. A file's group lets a whole team read it, and device
groups give access to hardware without root: `video` for cameras and GPUs,
`dialout` or `uucp` for serial ports, depending on the distro.

Adding a person to a group is one change, in one place, instead of a change
on every file they need.

### su or sudo?

`su` asks for root's password and opens a root shell. `sudo` asks for your
own password, runs one command and logs it. Sharing one root password
between several people is the problem `sudo` solves. Ubuntu locks root's
password, so `su` doesn't work there unless you set one.
