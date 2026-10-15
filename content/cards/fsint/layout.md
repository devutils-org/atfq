---
id: fsint.layout
kind: concept
title: How a filesystem stores a file
aliases: [filesystem, filesystems, file system, extents, extent, block size, superblock, directory entry, fragmentation, defrag, sparse file, sparse files]
hubs: [disk]
probes: [fs.mounts]
see_also: [fs.inodes-links, fsint.journal, hw-storage.block-layer]
commands: [stat, du, filefrag]
paths: []
---

## TL;DR

- A disk is a long row of blocks (usually 4 KiB). A filesystem decides
  which blocks hold which file, and keeps that map on the disk too.
- Each file has an inode: owner, rights, times, size, and where its data
  lives, as extents ("1,200 blocks starting at block 88,000").
- A folder is a small file listing names and inode numbers. The name is
  not part of the file itself ({{fs.inodes-links}}).

```
stat notes.txt              # inode, size, blocks used
filefrag -v big.iso         # its extents on the disk
```

## Pitfalls

- A 10-byte file still takes a whole block: thousands of tiny files use
  far more disk than their sizes add up to. `du` counts blocks, `ls -l`
  bytes.
- Sparse files (VM images, databases) look bigger in `ls -l` than in
  `du`: unwritten parts take no space until written. Copying them
  carelessly can fill a disk.
- Defragmenting rarely helps on SSDs, and on btrfs it un-shares blocks
  between snapshots, multiplying the space used.

## Deep dive

### How does the filesystem find free space?

It keeps a map of free blocks (bitmaps on ext4, B-trees on XFS and
btrfs). ext4 also delays the choice until the data is written out
(delayed allocation), so a file written in many small pieces still lands
in one long run of blocks. The superblock, copied in several places,
records the layout itself: block size, counts, where the maps are.

### What happens when I open /home/alice/notes.txt?

The kernel starts at the root folder's inode, reads its entries to find
`home`, then that folder's entries for `alice`, then `notes.txt`. Each
step is a lookup, cached in RAM afterwards (the dentry cache), which is
why the second `ls` of a large tree is so much faster.
