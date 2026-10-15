---
id: time.ntp
kind: concept
title: "Keeping the clock right: NTP"
aliases: [ntp, chrony, chronyd, timesyncd, systemd-timesyncd, clock drift, clock skew, time sync, time synchronization, timedatectl, chronyc, ptp, certificate not yet valid]
hubs: []
probes: [time.clock]
see_also: [time.clocks, crypto.certificates]
commands: [timedatectl, chronyc]
paths: [/etc/chrony.conf, /etc/systemd/timesyncd.conf]
---

## TL;DR

- Computer clocks drift, by seconds a day. NTP asks time servers and
  corrects the clock continuously, to within milliseconds over the
  Internet.
- Linux distros run one NTP client: `chronyd` (Fedora, RHEL) or
  `systemd-timesyncd` (Debian, Ubuntu, Arch).
- A wrong clock breaks things far from time: TLS certificates, two-factor
  codes, Kerberos, builds, distributed databases.

```
timedatectl                       # "System clock synchronized: yes"
timedatectl timesync-status       # with systemd-timesyncd
chronyc tracking                  # with chrony: offset and drift
```

## Pitfalls

- "Certificate is not yet valid" or "expired" on every site: check the
  date first ({{crypto.certificates}}).
- Two NTP clients at once fight over the clock: run one.
- `make: Clock skew detected`: files from a machine (or a shared folder)
  whose clock is ahead.
- In containers, the clock is the host's: fix time on the host.

## Deep dive

### How does NTP measure across a slow network?

The client notes when it sends a request; the server stamps when it
received it and when it replied; the client notes when the reply
arrives. From the four times it computes both the round trip and the
offset, assuming the trip takes as long each way. Repeating this with
several servers filters out the outliers and estimates the drift.

### When do milliseconds not suffice?

Trading, telecoms and some databases need microseconds. PTP (Precision
Time Protocol) gets there on a local network, with network cards that
timestamp packets in hardware. Clouds offer similar precision through
their hypervisors.
