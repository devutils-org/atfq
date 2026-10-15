---
id: remote.copy
kind: concept
title: "Copying files over SSH: scp, sftp and rsync"
aliases: [scp, sftp, sshfs, copy over ssh, copy to server, upload, download file]
hubs: []
probes: []
see_also: [cmd.rsync, remote.ssh-config]
commands: [scp, sftp, rsync, sshfs]
paths: []
---

## TL;DR

- Everything that copies over SSH uses the same keys and the same
  `~/.ssh/config` names: if `ssh db` works, `scp file db:` works.
- `scp` for a quick copy, `rsync` for folders and anything big (it resumes
  and only sends differences), `sshfs` to mount a remote folder.
- A path without a leading `/` after `host:` starts in your home folder
  there.

```
scp report.pdf db:/tmp/
scp -r db:logs/ ./logs-db/
rsync -avP project/ db:project/      # resumable, shows progress
```

## Pitfalls

- `rsync src/ dst` and `rsync src dst` differ: the trailing slash copies
  the folder's contents, not the folder ({{cmd.rsync}}).
- Interrupted `scp` starts over; `rsync -P` picks up where it stopped.
- Remote paths with spaces need quoting twice with old scp: once for your
  shell, once for the remote one. Recent OpenSSH no longer needs it.
- sshfs over a slow link makes every `ls` slow: copy what you work on.

## Deep dive

### What changed in scp?

The old scp protocol ran a shell command on the server and trusted its
output, which caused security bugs and odd quoting. Since OpenSSH 9.0,
`scp` speaks the SFTP protocol underneath, with the same command line.
`scp -O` brings back the old protocol for servers without SFTP.

### Why is rsync faster on the second run?

It compares sizes and dates, and for changed files, checksums of blocks,
then sends only the blocks that differ. Copying a 10 GiB folder again
after a few edits sends a few megabytes.
