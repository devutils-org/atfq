---
id: netmodel.ip
kind: concept
title: IP addresses, subnets and NAT
aliases: [ip address, ip addresses, subnet, subnets, cidr, netmask, nat, private ip, public ip, ipv4, ipv6]
hubs: [network]
probes: [net.interfaces]
see_also: [net.routes, netmodel.ethernet]
commands: [ip]
paths: []
---

## TL;DR

- An IPv4 address is 32 bits, written `192.168.1.23`. `/24` means the
  first 24 bits name the network and the rest the machine.
- `10.0.0.0/8`, `172.16.0.0/12` and `192.168.0.0/16` are private: used at
  home and at work, never on the Internet. Your router shares one public
  address through NAT.
- IPv6 addresses are 128 bits; `fe80::…` ones only work on the local link.
  There are enough to give every device a public address.

```
ip -br addr
curl -s https://ifconfig.me      # your public address, as others see it
```

## Pitfalls

- Two machines with the same address on one network fight over it: one
  of them becomes unreachable.
- An address in `169.254.x.x` means DHCP failed: the machine picked a
  link-local address by itself.
- `127.0.0.1` and `::1` are always the machine itself, never the network.
- Behind NAT, outside machines can't open a connection to yours unless the
  router forwards a port.

## Deep dive

### Why NAT?

IPv4 has about 4.3 billion addresses, fewer than there are devices. NAT
lets a whole home share one: the router rewrites addresses and ports on
the way out and remembers the mapping to send replies back. The cost is
that connections can't come in by themselves, which is why games and
peer-to-peer apps need port forwarding or tricks.

### How does a machine know where to send a packet?

It compares the destination with its own network. Same network: it sends
the packet directly, finding the MAC address with ARP. Anywhere else: it
sends it to the gateway. `ip route get ADDRESS` shows the choice
({{net.routes}}).
