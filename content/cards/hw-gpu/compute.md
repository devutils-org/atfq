---
id: hw-gpu.compute
kind: concept
title: "Computing on the GPU, and when your program can't see it"
aliases: [cuda, rocm, hip, opencl, nvidia-smi, rocm-smi, gpu compute, gpgpu, cuda_visible_devices, render group, render node, nvidia container toolkit, gpu not detected, torch.cuda.is_available]
hubs: []
probes: [gpu.devices]
see_also: [hw-gpu.basics, hw-gpu.drivers, perms.sudo-groups]
commands: [nvidia-smi, rocminfo, clinfo, groups]
paths: [/dev/dri/, /dev/kfd, /dev/nvidia0]
---

## TL;DR

- Compute frameworks talk to the GPU through a vendor stack: CUDA for
  NVIDIA, ROCm (HIP) for AMD, oneAPI for Intel. Vulkan and OpenCL work
  across vendors.
- Each needs the kernel driver, the matching user-space libraries, and
  access to device files: `/dev/nvidia*`, or `/dev/kfd` and
  `/dev/dri/renderD*`.
- When a program says "no GPU", check those three, in that order.

```
nvidia-smi                  # NVIDIA: driver, CUDA version, memory, users
rocminfo | grep -i name     # AMD
ls -l /dev/dri/; groups     # render nodes; are you in their group?
```

## Pitfalls

- Render nodes usually belong to the `render` group (sometimes `video`):
  add yourself, then log in again ({{perms.sudo-groups}}).
- Containers see no GPU unless given one: `--gpus all` with the NVIDIA
  Container Toolkit, or `--device /dev/kfd --device /dev/dri` for ROCm.
- A Python package built for CPU only, or for another CUDA version,
  reports no GPU though `nvidia-smi` works: install the build that matches.
- `CUDA_VISIBLE_DEVICES` or `HIP_VISIBLE_DEVICES` set to an empty or wrong
  value hides GPUs from a program.

## Deep dive

### What does "CUDA version" in nvidia-smi mean?

It's the newest CUDA version the installed driver supports, not a CUDA
installed on the machine. Programs bring their own CUDA runtime (PyTorch
wheels include it); they work as long as the driver is at least as new as
their CUDA needs. An old driver with a new framework is the usual cause
of "CUDA driver version is insufficient".

### What is a render node?

`/dev/dri/card0` controls the display, which only the compositor should
touch. `/dev/dri/renderD128` gives the same GPU for drawing and compute
only, with no control over screens, so it can be safely opened by
ordinary programs and containers. ROCm adds `/dev/kfd` for its compute
queues.
