---
id: git.merge-rebase
kind: concept
title: Merge or rebase?
aliases: [merge, rebase, git merge, git rebase, merge conflict, conflict, conflicts, fast-forward, interactive rebase, squash, cherry-pick, git cherry-pick]
hubs: []
probes: [git.config]
see_also: [git.model, git.remotes, git.undo]
commands: [git]
paths: []
---

## TL;DR

- Both bring another branch's work into yours. Merge adds a commit with
  two parents and keeps history as it happened. Rebase replays your
  commits on top of the other branch: a straight line.
- Rebase gives your commits new hashes. Rebase only commits nobody else
  has: your local or personal branches.
- A conflict means both sides changed the same lines: edit the file,
  `git add` it, then continue.

```
git merge main                 # or: git rebase main
git status                     # files in conflict
git merge --continue           # or: git rebase --continue / --abort
```

## Pitfalls

- After rebasing a branch you already pushed, a plain push is rejected;
  `git push --force-with-lease` replaces it, and breaks others' copies.
- `git rebase -i main` lets you squash, reorder or reword commits before
  sharing them; don't do it on shared history.
- Conflict markers left in a file (`<<<<<<<`) still commit: search for
  them before `git add`.
- `merge.conflictstyle = zdiff3` shows the common ancestor in conflicts:
  much easier to resolve.

## Deep dive

### What is a fast-forward?

If your branch has no commits the other lacks, there's nothing to merge:
git just moves your branch forward to the other's commit, with no merge
commit. That's what happens on `git pull` when you hadn't committed
locally. `--no-ff` forces a merge commit anyway, to keep a feature
visible as a group.

### Then which one?

Many teams rebase their own branch on the main one to keep it current
and readable, then merge it in (or squash it into one commit) through a
pull request. The rule that matters: never rewrite history that someone
else has based work on.
