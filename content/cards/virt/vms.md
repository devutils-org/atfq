---
id: virt.vms
kind: concept
title: Virtual machines and hypervisors
aliases: [virtual machine, virtual machines, vm, vms, hypervisor, kvm, qemu, virtualbox, vmware, virtio, wsl, wsl2, nested virtualization, hardware virtualization, vt-x, amd-v]
hubs: []
probes: [virt.detect, cpu.features]
see_also: [virt.containers, hw-cpu.instructions]
commands: [systemd-detect-virt, virsh, lscpu]
paths: [/dev/kvm]
---

## TL;DR

- A virtual machine runs a whole operating system, kernel included, on
  virtual hardware. The hypervisor shares the real CPUs, RAM and devices
  between machines.
- CPUs help directly (Intel VT-x, AMD-V, ARM's EL2): guest code runs at
  full speed, and only some operations go to the hypervisor.
- On Linux, KVM turns the kernel into a hypervisor; QEMU provides the
  virtual devices around it.

```
systemd-detect-virt          # none, kvm, vmware, docker…
grep -cw -E 'vmx|svm' /proc/cpuinfo   # 0: no hardware virtualization
ls -l /dev/kvm               # exists: this machine can run KVM guests
```

## Pitfalls

- No `/dev/kvm`: virtualization is off in the firmware settings, or you're
  already in a VM without nested virtualization.
- Emulated disks and network cards are slow; virtio devices, designed for
  VMs, are what to pick.
- A guest's clock and CPU numbers are what the hypervisor shows: CPU
  "steal" time in `top` (`st`) is time the host gave to others.
- WSL 2, Docker Desktop and OrbStack are Linux virtual machines; files
  shared with the host cross a slower boundary.

## Deep dive

### How does a CPU run a guest kernel safely?

Hardware virtualization adds a mode below the guest kernel. The guest
runs its own kernel code natively, but chosen events (touching certain
registers, some instructions, device access) exit to the hypervisor,
which handles them and resumes the guest. A second level of page tables
translates guest memory to host memory in hardware, so guest page tables
work untouched.

### Type 1 or type 2?

Type 1 hypervisors run on the bare machine (Xen, VMware ESXi, Hyper-V);
type 2 run as programs on an operating system (VirtualBox). KVM blurs it:
a module of an ordinary Linux kernel, with each VM an ordinary QEMU
process you can see in `top`.
