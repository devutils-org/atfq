---
id: git.remotes
kind: concept
title: "Remotes: fetch, pull and push"
aliases: [git remote, remote, origin, upstream, fetch, git fetch, pull, git pull, push, git push, rejected, non-fast-forward, force push, force-with-lease, tracking branch, divergent branches]
hubs: []
probes: [git.config]
see_also: [git.merge-rebase, remote.ssh-keys]
commands: [git]
paths: []
---

## TL;DR

- A remote is another copy of the repository, usually `origin` on GitHub
  or GitLab. Your copy is complete: most commands work offline.
- `fetch` downloads new commits without touching your branches; `pull`
  is fetch, then merge (or rebase) into your branch; `push` sends yours.
- `origin/main` is your last fetched view of the remote's `main`, not a
  live one.

```
git remote -v
git fetch && git status        # ahead or behind origin?
git branch -vv                 # each branch's upstream and distance
```

## Pitfalls

- "Updates were rejected": the remote has commits you don't. Pull (or
  fetch and rebase) first, then push again.
- "Need to specify how to reconcile divergent branches": choose once,
  `git config --global pull.rebase true` (or `false` to merge).
- Prefer `--force-with-lease` to `--force`: it refuses if the remote moved
  since your last fetch, instead of erasing someone's commits.
- `push.autoSetupRemote = true` saves the `-u origin BRANCH` on a new
  branch's first push.

## Deep dive

### HTTPS or SSH remotes?

With SSH (`git@github.com:user/repo.git`), your SSH key authenticates
you ({{remote.ssh-keys}}). With HTTPS, git asks a credential helper for a
token: the system keyring, or `store`, which writes it in plain text in
`~/.git-credentials`. Hosts no longer accept account passwords over HTTPS.

### What does the remote keep that I don't?

Nothing about your work: a clone has every commit and branch. What lives
only on the host are pull requests, issues and permissions. That's why
any clone can restore a lost server, and why a secret pushed once must be
revoked: removing it from history doesn't remove it from other clones.
