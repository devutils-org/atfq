---
id: fsint.vfs
kind: concept
title: "The VFS: one interface for every filesystem"
aliases: [vfs, virtual file system, virtual filesystem, fuse, network filesystem, nfs, smb, cifs, virtiofs, 9p, stale file handle]
hubs: [disk]
probes: [fs.mounts, kernel.overview]
see_also: [fs.proc-sys, remote.copy, fsint.layout]
commands: [findmnt, mount]
paths: [/proc/filesystems]
---

## TL;DR

- Programs call `open`, `read` and `write` the same way whatever holds
  the file. The kernel's VFS turns those calls into the right
  filesystem's code.
- So the same `cat` reads ext4 on a disk, NFS on a server, `/proc` made
  up by the kernel, or a FUSE filesystem written in Python.
- FUSE runs a filesystem as an ordinary program: sshfs, rclone, ntfs-3g,
  AppImages.

```
cat /proc/filesystems    # filesystems this kernel knows
findmnt -t nfs,nfs4,cifs,fuse.sshfs,virtiofs
```

## Pitfalls

- A network filesystem whose server vanishes can freeze every program
  touching it, in state D ({{proc.states}}): `hard` mounts wait forever.
- "Stale file handle" (NFS): the file was replaced or deleted on the
  server while you held it. Reopen it, or remount.
- FUSE is slower than a kernel filesystem: each operation goes to a
  program and back.
- File locking and inotify watches don't work reliably on every network
  filesystem.

## Deep dive

### What does the VFS keep in common?

The objects every filesystem must provide: superblocks (a mounted
filesystem), inodes (files), dentries (names in folders) and open file
structures, each with a table of operations the filesystem fills in. The
VFS handles path lookup, permissions, the page cache and caching of
names and inodes once, for all of them.

### How do VMs and containers share folders with the host?

virtiofs and 9p carry file operations between a guest and its host over
virtio. virtiofs is newer, much faster, and can share the host's page
cache. Docker Desktop and OrbStack mount the Mac's folders this way.
