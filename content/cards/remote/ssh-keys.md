---
id: remote.ssh-keys
kind: concept
title: SSH keys and the agent
aliases: [ssh, ssh key, ssh keys, ssh-keygen, ssh-agent, ssh-add, ssh-copy-id, id_ed25519, id_rsa, authorized_keys, passphrase, "permission denied (publickey)", unprotected private key file]
hubs: []
probes: [ssh.setup]
see_also: [remote.ssh-config, perms.rwx]
commands: [ssh-keygen, ssh-copy-id, ssh-add, ssh]
paths: [~/.ssh/, ~/.ssh/authorized_keys]
---

## TL;DR

- A key pair replaces your password: the private key stays on your
  machine, the public one (`.pub`) goes into `~/.ssh/authorized_keys` on
  each server.
- Protect the private key with a passphrase, and let an agent remember it
  for the session, so you type it once.
- Ed25519 is the type to create today: short, fast and safe.

```
ssh-keygen -t ed25519 -C "alice@laptop"
ssh-copy-id alice@server          # adds your public key over there
ssh -v alice@server               # shows which keys are tried, and why
```

## Pitfalls

- "Permissions 0644 for 'id_ed25519' are too open": ssh refuses a private
  key others can read. `chmod 600 ~/.ssh/id_ed25519`, and `700` on `~/.ssh`.
- "Permission denied (publickey)" with the right key: on the server,
  `~/.ssh` or `authorized_keys` writable by others makes sshd ignore it.
- Never copy the private key to a server to "jump" further: use
  `ProxyJump` ({{remote.ssh-config}}).
- Agent forwarding (`ssh -A`) lets anyone with root on that server use
  your keys while you're connected.

## Deep dive

### How does a key log you in without being sent?

The server picks a challenge built from the session; your client signs it
with the private key; the server checks the signature with the public key
it has in `authorized_keys`. Only someone holding the private key can
produce that signature, and the signature is useless for any other
session, so nothing reusable crosses the network.

### What does the agent do?

`ssh-agent` keeps decrypted keys in memory and signs challenges for ssh,
which reaches it through the socket named in `SSH_AUTH_SOCK`. Desktop
sessions usually start one. `AddKeysToAgent yes` in `~/.ssh/config` adds
a key the first time you use it; `ssh-add -l` lists what's loaded.
