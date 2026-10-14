---
id: perms.mac
kind: concept
title: SELinux and AppArmor
aliases: [selinux, apparmor, mac, mandatory access control, getenforce, setenforce, aa-status, avc denied, lsm]
hubs: [permissions]
probes: [security.lsm]
see_also: [perms.rwx, perms.acl-caps]
commands: [getenforce, setenforce, aa-status, restorecon]
paths: [/etc/selinux/config, /etc/apparmor.d/]
---

## TL;DR

- Both add a second set of rules on top of rwx: even a process running as
  root can only do what its policy allows.
- SELinux (Fedora, RHEL) labels every file and process. AppArmor (Ubuntu,
  Debian) gives profiles to programs, by path.
- A denial looks like a "Permission denied" that rwx can't explain: look in
  the audit log.

```
getenforce                       # SELinux: Enforcing or Permissive
sudo aa-status                   # AppArmor: profiles and their mode
sudo ausearch -m avc -ts recent  # recent SELinux denials
```

## Pitfalls

- Don't turn SELinux off to fix a problem. Switch to permissive for a test
  (`sudo setenforce 0`), find the denial, then fix the label or the policy.
- A file moved with `mv` keeps its old SELinux label; a copy gets the
  folder's. `sudo restorecon -Rv /path` puts the expected labels back.
- Serving web files from an unusual folder needs the right SELinux label
  or boolean (`setsebool`), on top of the usual rights.
- `sudo aa-complain PROFILE` makes one AppArmor profile log instead of
  block, to debug a single program.

## Deep dive

### Why another layer?

Classic rights are discretionary: an owner decides, and a hijacked program
gets every right of its user. Mandatory access control limits each program
to what it needs, whoever runs it: a compromised web server can't read
home folders, even running as root.

### Labels or paths?

SELinux puts a label on every file and process and writes rules between
labels: precise and hard to escape, but complex to read. AppArmor names
files by their path in a profile per program: much easier to write, but a
file reachable under another path can escape a rule.
