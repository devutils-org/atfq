---
id: kernel.modules
kind: concept
title: Kernel modules
aliases: [modules, module, kernel module, lsmod, modprobe, modinfo, driver, drivers, tainted, dkms, blacklist]
hubs: []
probes: [kernel.modules]
see_also: [boot.sequence, kernel.dmesg]
commands: [lsmod, modinfo, modprobe]
paths: [/proc/modules, /etc/modprobe.d/, /etc/modules-load.d/]
---

## TL;DR

- Most drivers are modules: pieces of the kernel loaded when the hardware
  that needs them shows up.
- `lsmod` lists the loaded ones, `modinfo` describes one, `modprobe` loads
  one with what it depends on.
- Module options and blacklists go in a `.conf` file in `/etc/modprobe.d/`.

```
lsmod | sort -k2 -n | tail
modinfo snd_hda_intel | head
echo 'blacklist pcspkr' | sudo tee /etc/modprobe.d/nobeep.conf
```

## Pitfalls

- Modules are built for one kernel version. After a kernel update, modules
  from outside the kernel (NVIDIA, VirtualBox) must be rebuilt: DKMS does it
  for you.
- A "tainted" kernel isn't broken: it runs code its developers can't debug,
  such as a proprietary module. Kernel developers usually ignore bug reports
  from tainted kernels.
- `blacklist` only stops automatic loading: the module can still load by
  hand or as a dependency. `install NAME /bin/false` blocks it completely.
- With Secure Boot on, the kernel refuses unsigned modules (see 9.1).

## Deep dive

### Who loads modules?

When a device appears, the kernel announces it with a "modalias", a string
that describes the hardware. udev hands it to modprobe, which loads the
module that claims it. Modules that have no hardware to wait for can be
listed in `/etc/modules-load.d/` to load at every boot.

### What do the taint letters mean?

`/proc/sys/kernel/tainted` is a set of flags; 0 means clean. The common
ones: `P` a proprietary module, `O` a module built outside the kernel tree,
`E` an unsigned module, `W` the kernel issued a warning, `D` the kernel hit
a fatal error (an "oops"). The full list is in the kernel documentation,
under "tainted kernels".
