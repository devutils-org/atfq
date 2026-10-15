---
id: virt.rootless
kind: concept
title: "Root in a container: users, capabilities and the docker group"
aliases: [rootless, rootless containers, rootless podman, docker group, privileged, --privileged, docker.sock, subuid, subgid, container escape, container security, userns-remap]
hubs: []
probes: [user.identity, virt.detect]
see_also: [virt.namespaces, perms.acl-caps, perms.sudo-groups]
commands: [id, podman, docker]
paths: [/etc/subuid, /etc/subgid, /run/docker.sock]
---

## TL;DR

- Without user namespaces, root in a container is root on the host, held
  back only by namespaces, dropped capabilities and seccomp.
- Being in the `docker` group equals being root: anyone in it can start a
  container with the host's `/` mounted and edit any file.
- Rootless containers (Podman by default, Docker in rootless mode) map
  container root to your own user: an escape gets only your rights.

```
id                                  # in the docker group?
podman run --rm alpine id           # uid=0 inside…
podman unshare cat /proc/self/uid_map   # …mapped to your uid outside
```

## Pitfalls

- `--privileged` gives every capability and every device: practically
  no isolation. Add only the capability you need (`--cap-add`).
- Mounting `/var/run/docker.sock` into a container hands it control of
  the host.
- Files a rootful container writes to a bind mount belong to root on the
  host; rootless ones get IDs from your range in `/etc/subuid`.
- Run the application as a non-root user inside the image (`USER` in the
  Dockerfile): defence in depth.

## Deep dive

### How does rootless work?

`/etc/subuid` gives your user a range of IDs, such as 100000 to 165535.
A user namespace maps container ID 0 to your own ID and 1 to 65535 to
that range, so images with several users still work. Networking can't
create real interfaces without privileges, so a user-space network stack
(slirp4netns or pasta) carries the traffic.

### What does root inside still have?

A reduced set of capabilities (Docker keeps about fourteen, such as
CHOWN and NET_BIND_SERVICE, and drops SYS_ADMIN, SYS_MODULE and others),
a seccomp filter blocking dangerous system calls, and AppArmor or
SELinux labels. Each layer has been bypassed at some point; together
they make escapes rare.
