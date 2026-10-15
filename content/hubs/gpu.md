---
id: gpu
kind: hub
title: GPU
aliases: [gpus, graphics, graphics card, video card]
probes: [gpu.devices]
commands:
  - { cmd: "lspci -k | grep -A3 -E 'VGA|3D'", desc: "GPUs and the kernel driver each uses", card: hw-gpu.drivers }
  - { cmd: "glxinfo -B", desc: "OpenGL renderer: your GPU, or llvmpipe for software (mesa-utils)", card: hw-gpu.drivers }
  - { cmd: "vulkaninfo --summary", desc: "Vulkan devices and drivers" }
  - { cmd: "nvidia-smi", desc: "NVIDIA: use, memory, processes, driver and CUDA version", card: hw-gpu.compute }
  - { cmd: "echo $XDG_SESSION_TYPE", desc: "wayland or x11", card: hw-gpu.display }
paths:
  - { path: "/sys/class/drm/", desc: "graphics cards and their screen outputs" }
  - { path: "/dev/dri/", desc: "card* to drive screens, renderD* to draw and compute" }
cards: [hw-gpu.basics, hw-gpu.drivers, hw-gpu.display, hw-gpu.compute]
---

Which graphics chip you have, which driver runs it, and whether programs
can use it.
