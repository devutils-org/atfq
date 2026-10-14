---
id: network
kind: hub
title: Network
aliases: [net, networking, ip, wifi, ethernet, interfaces]
probes: [net.interfaces]
commands:
  - { cmd: "ip -br addr", desc: "each interface and its addresses, one line each" }
  - { cmd: "ip route", desc: "where packets go, default gateway included" }
  - { cmd: "ping -c 3 example.com", desc: "is a host reachable, and how fast" }
  - { cmd: "ss -tulpn", desc: "ports open for connections", card: net.ports }
  - { cmd: "resolvectl status", desc: "DNS servers in use, per interface", card: net.dns }
  - { cmd: "nmcli device", desc: "NetworkManager's view, Wi-Fi included" }
  - { cmd: "curl -I https://example.com", desc: "test an HTTPS connection, headers only" }
paths:
  - { path: "/etc/hosts", desc: "names resolved without DNS" }
  - { path: "/etc/resolv.conf", desc: "DNS servers the system resolver asks" }
  - { path: "/sys/class/net/", desc: "one folder per interface: state, MAC, speed" }
cards: []
---

Your network interfaces, their addresses, where traffic goes, and how names
become addresses.
