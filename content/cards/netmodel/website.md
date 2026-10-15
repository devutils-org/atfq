---
id: netmodel.website
kind: concept
title: What happens when you open a website
aliases: [open a website, what happens when, http, https, tls, tls handshake, url, http/3]
hubs: [network]
probes: [net.layers]
see_also: [netmodel.layers, net.dns, netmodel.tcp-udp]
commands: [curl]
paths: []
---

## TL;DR

- DNS: the name becomes an IP address ({{net.dns}}).
- TCP: a three-way handshake opens a connection to port 443.
- TLS: client and server agree on encryption keys, and the server proves
  who it is with a certificate.
- HTTP: the browser sends a request, the server answers. With HTTP/3, the
  transport is QUIC, over UDP, instead of TCP.

```
curl -v -o /dev/null https://example.com     # every step, in order
```

## Pitfalls

- A slow first request is often DNS or the TLS handshake. `curl -w
  '%{time_namelookup} %{time_connect} %{time_appconnect}\n'` splits the
  time between them.
- A certificate error means a name or date that doesn't match, or an
  issuer that isn't trusted: check the machine's clock first.
- A proxy (`HTTPS_PROXY`) changes the whole path: `curl -v` shows it.

## Deep dive

### How does the packet leave your network?

Your machine sees the server isn't on its own network, so it sends the
packet to the gateway, whose MAC address it finds with ARP
({{netmodel.ethernet}}). The router replaces your private address with its
public one (NAT, {{netmodel.ip}}) and forwards the packet; each router on
the way picks the next hop from its own table.

### Why is HTTPS everywhere now?

Without TLS, anyone on the path, a café's Wi-Fi or an Internet provider,
can read and change the traffic. Free certificates from Let's Encrypt
(from the end of 2015) and browsers marking plain HTTP "not secure" (Chrome
in 2018) made HTTPS the default.
