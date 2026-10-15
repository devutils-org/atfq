---
id: hw-gpu.display
kind: concept
title: "How a picture reaches the screen: KMS, Wayland and X11"
aliases: [wayland, x11, xorg, xwayland, kms, modesetting, nomodeset, compositor, display server, screen resolution, refresh rate, tearing, vsync, black screen]
hubs: []
probes: [gpu.devices]
see_also: [hw-gpu.drivers, boot.cmdline]
commands: [echo, cat]
paths: [/sys/class/drm/]
---

## TL;DR

- The kernel sets up the screen itself (KMS: kernel mode setting):
  resolution, refresh rate, which output shows which image.
- Each window draws into its own buffer. A compositor combines them into
  one image and asks the kernel to show it at the next refresh.
- On Wayland the compositor is the display server (GNOME, KDE, Sway).
  On X11, the X server sits in between. Old X11 apps run on Wayland
  through XWayland.

```
echo $XDG_SESSION_TYPE                 # wayland or x11
cat /sys/class/drm/*/status            # which outputs have a screen
```

## Pitfalls

- `nomodeset` on the kernel command line disables KMS: a basic
  framebuffer with no acceleration. Useful to get a broken system to a
  desktop, never to keep ({{boot.cmdline}}).
- Screen sharing and global hotkeys work differently on Wayland: apps go
  through portals (PipeWire), not by reading the whole screen.
- Tearing is an image changed in the middle of a refresh. Compositors
  avoid it by flipping only between refreshes, at a small latency cost.

## Deep dive

### What happens 60 times a second?

When a window has a new frame, it hands the compositor a buffer, often
already in GPU memory. The compositor draws all visible windows into a
new screen buffer with the GPU, then asks the kernel to flip: show this
buffer from the next vertical blank, the pause between two refreshes.
The kernel tells it when the flip happened, and the cycle starts again.

### Why did Wayland replace X11?

X11's design dates from 1984: the server drew everything itself, over the
network if needed. Modern desktops bypassed most of it, leaving a complex
middleman with weak isolation: any X11 app can read the keyboard and the
screen of every other. Wayland makes the compositor the server, and each
app sees only its own windows.
