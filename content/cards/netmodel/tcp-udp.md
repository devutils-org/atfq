---
id: netmodel.tcp-udp
kind: concept
title: TCP, UDP and connections
aliases: [tcp, udp, quic, three-way handshake, syn, time_wait, retransmission, retransmissions, connection refused, connection timed out]
hubs: [network]
probes: [net.layers]
see_also: [net.ports, netmodel.website]
commands: [ss]
paths: [/proc/net/snmp]
---

## TL;DR

- TCP gives a reliable, ordered stream: it numbers the bytes, waits for
  acknowledgements and sends again what was lost.
- UDP sends independent datagrams with no promise of delivery: no waiting,
  used by DNS, video calls, games and QUIC (HTTP/3).
- A TCP connection starts with a three-way handshake (SYN, SYN-ACK, ACK)
  and is named by four values: both addresses and both ports.

```
ss -tan state established     # TCP connections open now
ss -s                         # how many, in each state
```

## Pitfalls

- "Connection refused": the host answered, but nothing listens on that
  port. "Timed out": no answer at all, often a firewall dropping packets.
- Many sockets in TIME_WAIT after closing is normal: the side that closed
  first keeps the pair for 60 seconds, so late packets can't confuse a new
  connection.
- Retransmissions mean lost packets. A small share is normal, especially on
  Wi-Fi; a steady high rate slows every transfer.

## Deep dive

### How does TCP know how fast to send?

Congestion control: it starts slowly, speeds up while acknowledgements
come back, and slows down when packets get lost. Linux uses CUBIC by
default; BBR, which watches delays rather than losses, is a common
alternative. `sysctl net.ipv4.tcp_congestion_control` shows yours.

### Why build QUIC on top of UDP?

A new transport protocol can't cross the Internet: firewalls and routers
let TCP and UDP through, little else. QUIC puts reliability, encryption and
several streams on top of UDP, inside the application, so it can change
without waiting for kernels and routers. And one lost packet no longer
stalls every stream, as it does with HTTP/2 over TCP.
