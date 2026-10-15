---
id: netmodel.layers
kind: concept
title: "Network layers: OSI and TCP/IP"
aliases: [osi, osi model, network layers, tcp/ip model, network stack, layers]
hubs: [network]
probes: [net.layers]
see_also: [netmodel.website, netmodel.ethernet, netmodel.tcp-udp]
commands: [ip, ping, dig, curl]
paths: []
---

## TL;DR

- A network is built in layers, each using the one below: link (Ethernet,
  Wi-Fi), network (IP), transport (TCP, UDP), application (HTTP, DNS, SSH).
- The OSI model names seven layers, the Internet uses four, and people
  still say "layer 2" for Ethernet and "layer 7" for HTTP.
- When something fails, check from the bottom up:

```
ip -br link                     # 2: is the link up?
ip -br addr; ip route           # 3: an address, and a way out?
ping -c 3 1.1.1.1               # 3: does IP reach the Internet?
dig example.com                 # 7: do names resolve?
curl -I https://example.com     # 4 to 7: TCP, TLS, HTTP
```

## Pitfalls

- `ping 1.1.1.1` works but `ping example.com` doesn't: it's DNS, not the
  network.
- `ping` uses ICMP, which some networks block: a failed ping doesn't prove
  a host is down.
- OSI layers 5 and 6 (session, presentation) have no clear place of their
  own on the Internet; TLS is usually put between 4 and 7.

## Deep dive

### Where does the OSI model come from?

From an ISO standard published in 1984, meant to define a complete set of
network protocols. Those protocols lost to TCP/IP, which already ran on
the ARPANET and in BSD Unix. The seven-layer vocabulary stayed, as the
common way to say where a problem sits.

### What does each layer add?

On the way out, each layer wraps what comes from above in its own header:
TCP adds ports and sequence numbers, IP adds the source and destination
addresses, Ethernet adds MAC addresses. The receiver unwraps them in
reverse order. A packet carrying a few bytes is mostly headers.
