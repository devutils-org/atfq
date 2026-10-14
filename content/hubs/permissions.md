---
id: permissions
kind: hub
title: Permissions
aliases: [perms, access rights, permission denied]
probes: [user.identity, security.lsm]
commands:
  - { cmd: "id", desc: "your user, your groups", card: perms.sudo-groups }
  - { cmd: "ls -l file", desc: "owner, group and rights of a file", card: perms.rwx }
  - { cmd: "namei -l /path/to/file", desc: "rights of every folder on the way to a file" }
  - { cmd: "sudo -l", desc: "what sudo lets you run", card: perms.sudo-groups }
  - { cmd: "getfacl file", desc: "extra users and groups allowed (ACL)", card: perms.acl-caps }
  - { cmd: "getenforce", desc: "SELinux mode, on Fedora and RHEL", card: perms.mac }
paths:
  - { path: "/etc/passwd", desc: "accounts: uid, home folder, shell" }
  - { path: "/etc/group", desc: "groups and their members" }
  - { path: "/etc/sudoers.d/", desc: "who may use sudo, and for what" }
cards: []
---

Who you are to the system, and what decides whether you may read, write or
run something.
