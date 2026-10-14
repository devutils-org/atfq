---
id: net.dns
kind: concept
title: DNS resolution
aliases: [dns, resolver, resolv.conf, nameserver, systemd-resolved, resolvectl, nsswitch, /etc/hosts, getent]
hubs: [network]
probes: [net.dns]
see_also: [net.ports]
commands: [getent, resolvectl, dig]
paths: [/etc/resolv.conf, /etc/nsswitch.conf, /etc/hosts]
---

## TL;DR

- Programs ask the system resolver to turn a name into an address. It tries
  the sources on the `hosts:` line of `/etc/nsswitch.conf`, in order.
- `files` means `/etc/hosts`; `dns` means the servers in `/etc/resolv.conf`.
- With `nameserver 127.0.0.53`, systemd-resolved does the work: ask it with
  `resolvectl`.

```
getent hosts example.com    # the answer your programs get
resolvectl status           # the real DNS servers, per interface
dig example.com             # ask a DNS server directly
```

## Pitfalls

- `dig` and `nslookup` skip `/etc/hosts` and nsswitch: they can disagree
  with your browser. `getent hosts` gives the system's answer.
- Don't edit `/etc/resolv.conf` when systemd-resolved or NetworkManager
  manages it: your change gets overwritten.
- Old answers can stay cached after a change: `resolvectl flush-caches`.
- Browsers may use their own DNS over HTTPS and skip the system resolver.

## Deep dive

### Why is the nameserver 127.0.0.53?

That's systemd-resolved's local stub. Programs send it their questions; it
caches answers and forwards the rest to the real servers it learned from
DHCP or your settings. It can use different servers per interface, so names
of a company network go to the VPN's DNS and the rest to your usual one.

### What's on the hosts line?

The sources, tried in order. A typical systemd line is `mymachines` (local
containers), `resolve` (systemd-resolved), `files` (`/etc/hosts`),
`myhostname` (your own hostname always resolves) and `dns`. The
`[!UNAVAIL=return]` after `resolve` means: if systemd-resolved is running,
take its answer and stop there.
