---
id: git.config
kind: concept
title: Configuring git
aliases: [git config, gitconfig, .gitconfig, git alias, git aliases, user.email, user.name, credential helper, includeif, .gitignore, gitignore, autocrlf]
hubs: []
probes: [git.config]
see_also: [git.remotes, shell.env]
commands: [git]
paths: [~/.gitconfig, ~/.config/git/config, /etc/gitconfig, .git/config]
---

## TL;DR

- Three levels: system (`/etc/gitconfig`), yours (`~/.gitconfig`) and the
  repository's (`.git/config`). The closest one wins.
- Set your name and email once; commits carry them forever.
- `.gitignore` lists files git should never track: build output,
  secrets, editor files.

```
git config --global user.name "Alice Example"
git config --global user.email alice@example.com
git config --list --show-origin     # every setting, and its file
```

## Pitfalls

- A work and a personal email: `[includeIf "gitdir:~/work/"]` loads a
  second file for repositories under `~/work/`.
- `.gitignore` doesn't affect files already committed: `git rm --cached
  FILE` stops tracking one, keeping it on disk.
- `credential.helper = store` keeps tokens in plain text; use your
  desktop keyring (`libsecret`) or SSH keys instead.
- On Linux, `core.autocrlf` should be `input` or unset; `true` is for
  Windows.

## Deep dive

### Which settings are worth setting?

`init.defaultBranch` (the first branch's name), `pull.rebase` (how pull
integrates), `push.autoSetupRemote`, `merge.conflictstyle = zdiff3`,
`rebase.autoStash` (stash and restore around a rebase) and `fetch.prune`
(forget branches deleted on the remote). Aliases (`git config --global
alias.st "status -sb"`) shorten what you type daily.

### Where do personal ignores go?

Files only you produce, like your editor's, belong in
`~/.config/git/ignore`, read for every repository. The project's
`.gitignore` is for what every contributor generates.
