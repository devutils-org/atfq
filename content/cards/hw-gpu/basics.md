---
id: hw-gpu.basics
kind: concept
title: What a GPU does differently
aliases: [gpu, gpus, graphics card, video card, vram, video memory, integrated graphics, discrete gpu, igpu, dgpu, simt, shader, shaders]
hubs: []
probes: [gpu.devices]
see_also: [hw-gpu.compute, hw-cpu.cores]
commands: [lspci]
paths: [/sys/class/drm/]
---

## TL;DR

- A CPU has a few powerful cores, built to run one thread fast, branches
  and all. A GPU has thousands of simple ones, built to run the same
  instructions on huge amounts of data at once.
- That fits pixels, video and the matrix maths of machine learning; it
  doesn't fit branchy, step-by-step code.
- A discrete GPU has its own memory (VRAM), very fast but separate:
  data must be copied there over PCIe. An integrated GPU shares the
  system's RAM.

```
lspci -k | grep -A3 -E 'VGA|3D|Display'   # GPUs and their drivers
```

## Pitfalls

- Copying data between RAM and VRAM is often the bottleneck: a GPU only
  pays off when it does a lot of work per byte copied.
- Running out of VRAM fails or slows down hard; it isn't extended by swap
  the way RAM is.
- Laptops with two GPUs run most programs on the integrated one; the
  discrete one has to be requested (`prime-run`, or the desktop's "run with
  dedicated GPU" option).

## Deep dive

### How does a GPU run thousands of threads?

Threads are grouped by 32 (NVIDIA "warps") or 32 to 64 (AMD
"wavefronts"), and a group runs one instruction at a time for all its
threads, each on its own data. When threads of a group take different
branches, the group runs both paths in turn: that's why branches are
expensive. While a group waits on memory, the GPU switches to another at
no cost: it hides latency with parallelism, where a CPU uses big caches.

### Why is VRAM so fast?

It's built for bandwidth: GDDR memory on wide buses, or HBM stacked next
to the chip, reaches hundreds of GB/s to several TB/s, against tens of
GB/s for a PC's RAM. The price is size and cost, which is why the
amount of VRAM limits the models and scenes a GPU can handle.
