---
id: hw-gpu.drivers
kind: concept
title: "GPU drivers: the kernel, Mesa and NVIDIA"
aliases: [gpu driver, gpu drivers, graphics driver, graphics drivers, nvidia driver, mesa, nouveau, amdgpu, i915, xe, proprietary driver, drm, opengl, vulkan, glxinfo, vulkaninfo, driver/library version mismatch]
hubs: []
probes: [gpu.devices, kernel.modules]
see_also: [kernel.modules, hw-gpu.display, boot.cmdline]
commands: [lspci, glxinfo, vulkaninfo, modinfo]
paths: [/sys/class/drm/, /proc/driver/nvidia/version]
---

## TL;DR

- A GPU driver has two halves: a kernel driver (DRM) that owns the device
  and its memory, and a user-space library that turns OpenGL or Vulkan
  calls into GPU commands.
- Intel (`i915`, `xe`) and AMD (`amdgpu`) are in the kernel, and Mesa
  provides their OpenGL and Vulkan: nothing to install.
- NVIDIA's own driver ships a kernel module and libraries that must match
  exactly. `nouveau` is the open driver, slower on recent cards.

```
lspci -k | grep -A3 -E 'VGA|3D'       # "Kernel driver in use"
glxinfo -B                            # OpenGL renderer (mesa-utils)
vulkaninfo --summary
```

## Pitfalls

- "Driver/library version mismatch" after an NVIDIA update: the old
  module is still loaded. Reboot.
- A kernel update can leave NVIDIA's module unbuilt (DKMS failed): the
  desktop falls back to a basic driver, or a black screen.
- With Secure Boot, out-of-tree modules must be signed (enrolled MOK key)
  or the kernel refuses them ({{boot.loaders}}).
- `llvmpipe` as the OpenGL renderer means software rendering: no GPU
  driver is working.

## Deep dive

### What does each half do?

The kernel side sets screen modes, manages video memory, and schedules
command buffers from many programs safely: one program can't read
another's buffers. The user side, Mesa or NVIDIA's libraries, does the
heavy lifting: compiling shaders into the GPU's instruction set and
building the command buffers. Vulkan and OpenGL find it through loader
files in `/usr/share/vulkan/icd.d/` and `glvnd`.

### Why is NVIDIA different?

NVIDIA long kept its kernel driver closed, so it can't be part of Linux
and is built against each kernel separately. Since 2022 its kernel
modules exist as open source (the default for recent GPUs), but the
user-space libraries stay closed and versioned with them. Meanwhile,
Mesa's NVK brings an open Vulkan driver for NVIDIA cards, on top of
nouveau.
