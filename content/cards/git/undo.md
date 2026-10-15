---
id: git.undo
kind: concept
title: Undoing things in git
aliases: [git undo, git reset, git revert, git restore, git reflog, reflog, undo commit, git commit --amend, amend, lost commit, discard changes, git stash, stash, unstage]
hubs: []
probes: []
see_also: [git.model, git.remotes]
commands: [git]
paths: []
---

## TL;DR

- Committed work is almost never lost: the reflog remembers every place a
  branch has been. Uncommitted edits are the fragile part.
- Pick the command by what you want back: a file, a commit, or a commit
  others already have.

```
git restore --staged FILE     # unstage a file, keep the edits
git restore FILE              # drop your edits to it: gone for good
git reset --soft HEAD~1       # undo the last commit, keep its changes
git revert COMMIT             # undo a pushed commit with a new commit
git reflog                    # where HEAD has been: find lost commits
```

## Pitfalls

- `git reset --hard`, `git restore`, `git checkout -- FILE` and
  `git clean -f` throw away uncommitted work: git has no copy of it.
- Don't amend or reset commits others already pulled: use `git revert`,
  which undoes by adding a commit.
- `git stash` before a risky command keeps your edits aside; `git stash
  pop` brings them back.
- `git clean -n` lists what `git clean -f` would delete.

## Deep dive

### How can the reflog save me?

Every time `HEAD` moves (commit, reset, rebase, checkout), git writes the
old and new positions in the reflog, for 90 days by default. A commit
"lost" by a bad reset or rebase is still in the object store: find it in
`git reflog`, then `git branch rescue ID` or reset to it. Only
uncommitted changes are truly unrecoverable.

### reset, restore, revert: which is which?

`restore` changes files (working tree or staging area) and never moves a
branch. `reset` moves the current branch to another commit; `--soft`
keeps your changes staged, the default keeps them unstaged, `--hard`
discards them. `revert` creates a new commit that applies the opposite
of an old one, leaving history as it was.
