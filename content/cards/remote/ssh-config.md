---
id: remote.ssh-config
kind: concept
title: "~/.ssh/config, jump hosts and host keys"
aliases: [ssh config, .ssh/config, ssh_config, proxyjump, jump host, bastion, known_hosts, host key, host key verification failed, remote host identification has changed, controlmaster]
hubs: []
probes: [ssh.setup]
see_also: [remote.ssh-keys, remote.tunnels]
commands: [ssh, ssh-keygen]
paths: [~/.ssh/config, ~/.ssh/known_hosts, /etc/ssh/sshd_config]
---

## TL;DR

- `~/.ssh/config` gives each server a short name with its address, user,
  port and key: `ssh db` instead of a long command line.
- `ProxyJump` reaches a server through another one (a bastion), end to
  end encrypted, with your keys staying on your machine.
- The first connection asks you to trust the server's host key and saves
  it in `known_hosts`; later, a different key stops the connection.

```
Host db
    HostName 10.0.4.12
    User alice
    ProxyJump bastion.example.com
```

## Pitfalls

- ssh keeps the first value it finds for each option: put specific
  `Host` blocks first and `Host *` last.
- "REMOTE HOST IDENTIFICATION HAS CHANGED": the server was reinstalled, or
  someone is in between. Check with its admin, then remove the old key:
  `ssh-keygen -R host`.
- Changing the server's port or disabling passwords in `sshd_config`:
  keep a working session open while you test a second one.
- `ssh -G host` prints the options that apply after reading every config
  file: the quickest way to see why a setting is ignored.

## Deep dive

### Why trust a key on first use?

Nothing tells ssh in advance what a server's key should be, so it asks
you once and remembers. That protects every later connection: an
attacker would have to be in the middle on the very first one. Admins
can publish fingerprints, or sign host keys with a certificate, to
remove even that first doubt.

### How do I make repeated connections faster?

`ControlMaster auto`, `ControlPath ~/.ssh/cm-%C` and `ControlPersist 10m`
keep one connection open and run every later ssh, scp or git command
through it: no new handshake, no new authentication. Useful with slow
jump hosts or two-factor logins.
