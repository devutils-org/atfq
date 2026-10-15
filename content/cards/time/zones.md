---
id: time.zones
kind: concept
title: Time zones, UTC and daylight saving
aliases: [timezone, time zone, time zones, utc, dst, daylight saving, daylight saving time, tz, tzdata, localtime, /etc/localtime, set timezone, gmt]
hubs: []
probes: [time.clock]
see_also: [time.epoch, time.clocks]
commands: [timedatectl, date]
paths: [/etc/localtime, /usr/share/zoneinfo/]
---

## TL;DR

- The kernel counts time in UTC. Time zones are applied when time is
  displayed, by the C library, from the rules in `/etc/localtime`.
- Each process can use another zone: `TZ=Asia/Tokyo date`.
- Store and compute in UTC; convert to local time only to show it.

```
timedatectl set-timezone Europe/Paris
TZ=America/New_York date
date -u                          # UTC
```

## Pitfalls

- When clocks go back, a local time like 02:30 happens twice; when they
  go forward, it doesn't happen at all. Jobs scheduled then may run
  twice or be skipped: schedule them outside 01:00–03:00, or in UTC.
- A day isn't always 24 hours, in local time: add days with a date
  library, not `+ 86400`.
- Zone rules change by law, sometimes at short notice: keep `tzdata`
  updated, and use names like `Europe/Paris`, not offsets like `+01:00`.
- Dual boot with Windows: Windows keeps the hardware clock in local
  time, Linux in UTC, so one of them shows the wrong hour.

## Deep dive

### What is in a zone file?

The zone's history: every change of offset and of daylight saving rules
since records began, compiled from the IANA time zone database. That's
why converting a date from 1975 in Europe/Paris gives the offset in use
then, and why `Europe/Paris` and "UTC+1" aren't the same thing.

### What's the difference between UTC and GMT?

GMT is a time zone (Britain in winter). UTC is the reference timescale,
kept by atomic clocks, with leap seconds added so it stays close to the
Earth's rotation ({{time.epoch}}). For everyday use they show the same
time.
