---
id: netmodel.ethernet
kind: concept
title: Ethernet, MAC addresses and ARP
aliases: [mac address, mac, ethernet, arp, neighbour, neighbor, ip neigh, switch, layer 2]
hubs: [network]
probes: [net.layers]
see_also: [netmodel.ip, netmodel.layers]
commands: [ip]
paths: [/proc/net/arp]
---

## TL;DR

- On a local network, frames travel from MAC address to MAC address:
  48-bit hardware addresses such as `f4:92:bf:0e:21:7a`.
- To reach an IP address on the same network, a machine asks "who has this
  IP?" (ARP) and keeps the answer in its neighbour table.
- A switch learns which MAC address sits behind each port and sends frames
  only there; Wi-Fi does the same through the access point.

```
ip neigh          # the neighbour (ARP) table
ip -br link       # your MAC addresses
```

## Pitfalls

- Phones and laptops often use a random MAC address per Wi-Fi network, for
  privacy: a router may list the same device twice.
- Two machines answering ARP for one address, by mistake or as an attack,
  make connections drop on and off.
- IPv6 doesn't use ARP: neighbour discovery (NDP) does the same job, with
  ICMPv6.

## Deep dive

### What's in a MAC address?

Six bytes. The first three often name the manufacturer, the rest the
device. One bit of the first byte marks "locally administered" addresses,
which is what random MAC addresses set. `ff:ff:ff:ff:ff:ff` is broadcast:
every machine on the link reads it.

### Why both MAC and IP addresses?

IP addresses are logical: they depend on the network you join, and they
carry a packet across many networks. MAC addresses deliver it on the
current link, one hop at a time. At each router, the IP addresses stay the
same while the MAC addresses change.
