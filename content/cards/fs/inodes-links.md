---
id: fs.inodes-links
kind: concept
title: Links, inodes and "disk full"
aliases: [inode, inodes, hard link, symlink, symbolic link, ln, links, no space left, disk full, df -i]
hubs: [disk]
probes: [disk.tree]
see_also: [fs.hierarchy]
commands: [ln, df, lsof]
paths: []
---

## TL;DR

- An inode is a file's record: owner, rights, size, where the data is.
  The name is stored in the folder, not in the inode.
- A hard link is a second name for the same inode. A symbolic link is a
  small file that holds a path.
- A filesystem can run out of inodes before space: "No space left on
  device" while `df -h` still shows room.

```
df -i                       # inode use per filesystem
ls -li                      # inode number of each file
ln -s /data/photos photos   # a symbolic link named photos
```

## Pitfalls

- `rm` removes a name. The space comes back when the last name is gone and
  no program has the file open: list deleted but open files with
  `sudo lsof +L1`.
- Millions of tiny files (caches, `node_modules`, mail) eat inodes. On ext4
  their number is fixed when the filesystem is created.
- btrfs has no fixed inode count (`df -i` shows 0), and XFS creates inodes
  as needed: inode exhaustion is mostly an ext4 problem.
- A hard link can't cross filesystems or point to a folder. A symbolic link
  can, but it breaks when its target moves.

## Deep dive

### Why is the name not part of the file?

A folder is a list of pairs: name and inode number. That's why renaming a
file within a filesystem is instant (only the folder changes), why one file
can have several names, and why a file has no single "real" path.

The inode counts its names: it's the number in the second column of
`ls -l`. When it drops to zero and nobody has the file open, the data is
freed.

### Why does root still have space when the disk is full?

ext4 keeps 5% of its blocks for root by default, so system services keep
working when users fill the disk. That's why `df` shows less available
space than size minus used. `tune2fs -m` changes the share, which can be
lowered on a big data disk that holds no system files.
