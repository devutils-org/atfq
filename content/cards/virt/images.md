---
id: virt.images
kind: concept
title: Images, layers and overlayfs
aliases: [container image, docker image, image layers, overlayfs, overlay, overlay2, dockerfile, docker build, volume, volumes, docker volume, bind mount, docker system df]
hubs: []
probes: [virt.detect]
see_also: [virt.containers, fs.fstab]
commands: [docker, podman]
paths: [/var/lib/docker/, /var/lib/containers/]
---

## TL;DR

- An image is a stack of read-only layers, one per build step, plus some
  settings (command, environment). Layers are shared between images.
- Running it adds a thin writable layer on top. overlayfs merges them
  into one root filesystem: you see the top-most version of each file.
- Data you want to keep belongs in a volume or a bind mount, outside
  the container's layers.

```
docker history IMAGE           # its layers and their sizes
docker system df               # space used by images, containers, volumes
docker run -v data:/var/lib/postgresql/data postgres
```

## Pitfalls

- Deleting a file in a later Dockerfile step doesn't shrink the image:
  the earlier layer still holds it. Download, use and delete in one `RUN`.
- Changing an early step rebuilds every step after it: copy dependency
  lists and install them before copying the source code.
- A tag such as `postgres:16` moves when it's updated; a digest
  (`@sha256:…`) never does.
- Images and stopped containers pile up: `docker system df`, then prune
  what you don't need.

## Deep dive

### How does overlayfs work?

It mounts several folders as one: read-only lower layers and one
writable upper layer. Reading looks through from the top down. The first
write to a lower file copies the whole file up first ("copy-up"), which
is slow for big files. Deleting leaves a "whiteout" marker in the upper
layer that hides the file below. Your `mount` shows each running
container's overlay.

### What's in an image file?

An OCI image is a manifest listing layers (tar archives, compressed)
and a config. Every part is named by its SHA-256 digest, so a registry
stores and sends each layer once, and pulling an image only downloads
the layers you don't have yet.
