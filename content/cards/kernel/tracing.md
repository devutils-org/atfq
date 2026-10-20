---
id: kernel.tracing
kind: concept
title: Why a tool may not watch the kernel
aliases: [perf_event_paranoid, paranoid, cap_bpf, cap_perfmon, unprivileged_bpf_disabled, kptr_restrict, lockdown, kernel lockdown, btf, memlock, rlimit_memlock, ebpf permission denied, perf permission denied, bpf operation not permitted, tracing permissions]
hubs: []
probes: [kernel.tracing]
see_also: [kernel.sysctl, perms.acl-caps, kernel.strace, concurrency.limits]
commands: [sysctl, setcap, getcap, ulimit]
paths: [/proc/sys/kernel/perf_event_paranoid, /sys/kernel/security/lockdown, /sys/kernel/btf/vmlinux]
---

## TL;DR

- Profilers and eBPF tools (`perf`, `bpftrace`, Omniscient) ask the kernel to
  watch other code. A few settings decide whether an ordinary user may.
- `kernel.perf_event_paranoid` limits perf events: at 2, the default on most
  distros, you can profile your own programs in user space only.
- `CAP_PERFMON` and `CAP_BPF` lift those limits for one program, without
  running it as root.
- Lockdown, a missing BTF file or a small locked-memory limit can still stop
  a tool that holds both.

```
sysctl kernel.perf_event_paranoid kernel.unprivileged_bpf_disabled
cat /sys/kernel/security/lockdown        # the mode in brackets
ls -l /sys/kernel/btf/vmlinux            # present: portable eBPF can load
sudo setcap cap_perfmon,cap_bpf+ep ./mytool   # getcap to check
```

## Pitfalls

- A file capability is lost when the file changes: rebuilding the binary
  drops it, so set it again after every build.
- Debian and Ubuntu patch the kernel so that a paranoid level of 3 or 4
  refuses perf to anyone without `CAP_PERFMON`, even on their own programs.
- On kernels before 5.11, eBPF maps are charged to the locked-memory limit:
  with the old 64 KiB default, loading a map fails with "operation not
  permitted", which looks like a missing permission.
- Lowering `perf_event_paranoid` affects every user of the machine. A
  capability on one binary is the narrower choice.

## Deep dive

### What does each paranoid level allow?

At -1, every event, for everyone. At 0, everything except raw tracepoints. At
1, your own processes, kernel side included, but no CPU-wide event. At 2,
your own processes in user space only. Higher values are distro patches.
`CAP_PERFMON` (Linux 5.8 and later) bypasses all of them; before 5.8 only
`CAP_SYS_ADMIN` did.

### What is BTF, and why do eBPF tools need it?

BTF is the kernel's own description of its data structures, shipped in
`/sys/kernel/btf/vmlinux`. Portable eBPF programs (CO-RE) read it to find
where a field sits in this kernel's build. Kernels built without
`CONFIG_DEBUG_INFO_BTF`, such as Ubuntu 18.04's 4.15, don't have the file,
and such programs cannot load there.

### What does lockdown block?

Lockdown protects the running kernel, usually when Secure Boot is on. In
`integrity` mode, nothing may modify it: no unsigned modules, no writes to
`/dev/mem`. In `confidentiality` mode, nothing may read its memory either:
kprobes, eBPF programs that read kernel memory, perf and tracefs are
refused, whatever the capabilities.
