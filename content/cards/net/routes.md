---
id: net.routes
kind: concept
title: Interfaces and routes
aliases: [route, routes, routing, gateway, default gateway, ip route, ip addr, interface, routing table]
hubs: [network]
probes: [net.interfaces]
see_also: [net.dns, net.firewall]
commands: [ip, nmcli]
paths: [/sys/class/net/, /proc/net/route]
---

## TL;DR

- An interface is a network connection point: `enp3s0` (cable), `wlp2s0`
  (Wi-Fi), `lo` (the machine itself).
- The routing table says where each packet goes: straight to a local
  network, or to the default gateway (your router) for everything else.
- `ip` shows and changes both; what you change with it is lost at reboot.

```
ip -br addr
ip route
ip route get 1.1.1.1     # which way a packet would take
```

## Pitfalls

- NetworkManager or systemd-networkd manage the network on most systems:
  change settings with `nmcli` or their files, or they undo your `ip`
  commands.
- Names like `enp3s0` come from the card's position in the machine; older
  guides say `eth0`.
- With two default routes (Wi-Fi and cable, or a VPN), the lowest metric
  wins.
- Docker, VPNs and VMs add their own interfaces and routes (`docker0`,
  `tun0`, `virbr0`): that's normal.

## Deep dive

### How is a route chosen?

The most specific route that matches wins: a route for 192.168.1.0/24
beats the default 0.0.0.0/0. Between equally specific routes, the lowest
metric wins. `ip route get ADDRESS` shows the result for one destination,
interface and source address included.

### What's in an interface name?

The prefix says the kind: `en` for Ethernet, `wl` for Wi-Fi. The rest says
where the card sits: `p3s0` is PCI bus 3, slot 0; `o1` is the first onboard
port. udev builds these names so that adding a card doesn't rename the
others, which happened with `eth0` and `eth1`.
