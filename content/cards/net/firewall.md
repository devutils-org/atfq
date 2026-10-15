---
id: net.firewall
kind: concept
title: nftables vs iptables vs ufw
aliases: [firewall, iptables, nftables, nft, ufw, firewalld, firewall-cmd, blocked port]
hubs: [network]
probes: [net.firewall]
see_also: [net.ports, net.routes]
commands: [nft, ufw, firewall-cmd]
paths: [/etc/nftables.conf, /etc/ufw/]
---

## TL;DR

- The kernel does the filtering (netfilter). iptables, nftables, ufw and
  firewalld are ways to write its rules.
- nftables (`nft`) replaced iptables. On current distros, the `iptables`
  command often writes nftables rules underneath.
- ufw (Ubuntu) and firewalld (Fedora, RHEL) are simpler front-ends: use the
  one your distro ships, and only that one.

```
sudo ufw status verbose
sudo firewall-cmd --list-all
sudo nft list ruleset
```

## Pitfalls

- Several tools at once write rules that fight each other: pick one
  front-end.
- Over SSH, allow SSH before turning a firewall on (`sudo ufw allow
  OpenSSH`), or you lock yourself out.
- Docker writes its own rules, so ports it publishes bypass ufw (see {{net.ports}}).
- Rules added by hand with `nft` or `iptables` vanish at reboot unless your
  distro's service saves them.

## Deep dive

### Why did nftables replace iptables?

iptables needed separate tools for IPv4, IPv6, ARP and bridges, and
replaced its whole rule table for every change. nftables has one tool and
one syntax for all of them, sets and maps for long lists of addresses, and
atomic updates. It arrived in kernel 3.13 and became the default in Debian
10 and RHEL 8.

### What are firewalld zones?

firewalld groups interfaces and networks into zones (`public`, `home`,
`trusted`), each with its own rules: a laptop on café Wi-Fi sits in
`public`. Changes made without `--permanent` are lost at the next reload,
which makes it easy to try a rule first.
