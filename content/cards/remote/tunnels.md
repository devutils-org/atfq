---
id: remote.tunnels
kind: concept
title: SSH tunnels and port forwarding
aliases: [ssh tunnel, ssh tunnels, port forwarding, local forwarding, remote forwarding, socks proxy, tunnel]
hubs: []
probes: [net.listening]
see_also: [remote.ssh-config, net.ports]
commands: [ssh, ss]
paths: []
---

## TL;DR

- `-L`: a port on your machine leads to a port reachable from the server.
  Reach a database that only listens on the server's localhost.
- `-R`: a port on the server leads back to your machine. Show a local
  web app to someone who can reach the server.
- `-D`: a SOCKS proxy; your browser's traffic leaves from the server.

```
ssh -N -L 5433:localhost:5432 db       # local 5433 → db's PostgreSQL
ssh -N -R 8080:localhost:3000 server   # server's 8080 → your 3000
ssh -N -D 1080 server                  # SOCKS proxy on localhost:1080
```

## Pitfalls

- `localhost` in `-L 5433:localhost:5432` means the server's localhost:
  the address is resolved at the far end.
- Forwarded ports listen on 127.0.0.1 only, on both sides, unless you
  give an address (and, for `-R`, the server allows `GatewayPorts`).
- `-N` runs no command, so the session looks frozen: that's the tunnel
  working. Ctrl-C closes it.
- "bind: Address already in use": something already listens on that
  local port ({{net.ports}}).

## Deep dive

### What travels where?

Your client accepts a connection on the local port, sends its bytes
inside the encrypted SSH session, and sshd opens a connection to the
target from the server's side. The target sees a connection from the
server; only the leg between client and server is protected by SSH, the
last hop is ordinary traffic.

### Tunnel or VPN?

A tunnel forwards one port per option, for one person, using an account
you already have. A VPN (WireGuard, OpenVPN) carries all traffic of whole
networks and needs setting up on both ends. For reaching one service now,
a tunnel; for a team reaching many, a VPN.
