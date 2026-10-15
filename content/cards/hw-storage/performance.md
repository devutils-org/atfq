---
id: hw-storage.performance
kind: concept
title: "IOPS, throughput and latency: how fast is a disk?"
aliases: [iops, disk speed, disk performance, disk latency, throughput, iostat, fio, iotop, queue depth, slow disk, io wait, iowait]
hubs: [disk]
probes: [disk.io]
see_also: [hw-storage.devices, proc.states]
commands: [iostat, fio, iotop]
paths: [/proc/diskstats]
---

## TL;DR

- Three numbers describe a disk: latency (time for one request), IOPS
  (requests per second) and throughput (bytes per second). Throughput =
  IOPS × request size.
- Small random requests show IOPS and latency; big sequential ones show
  throughput. A disk can be great at one and poor at the other.
- Measure your own pattern: `fio` reproduces it, `iostat -x` shows what
  a running system does.

```
iostat -x 1          # r_await/w_await: ms per request; aqu-sz: queue
sudo iotop -o        # who is doing I/O now
fio --name=t --rw=randread --bs=4k --size=1G --direct=1 --iodepth=32
```

## Pitfalls

- `%util` in iostat is the share of time with any request in flight. An
  NVMe drive at 100 % may still have plenty of capacity left.
- `dd` with no `oflag=direct` or `conv=fsync` measures the page cache,
  not the disk.
- A process stuck in state `D` with high `wa` in `top` is waiting for the
  disk, not using the CPU ({{proc.states}}).
- The fio command above creates a 1 GiB test file in the current folder:
  run it on the disk you want to test, and delete the file after.

## Deep dive

### What is queue depth?

The number of requests in flight at once. A hard drive gains little from
more than a few: it has one head. An SSD has many flash chips working in
parallel, so it only reaches its rated IOPS with dozens of requests
queued. A program that reads one block, waits, then reads the next uses
a depth of one, and sees a fraction of the box's figures.

### How can disk time add up to more than the clock?

The counters in /proc/diskstats add the time of every request. With 32
requests in flight for 1 second, the total says 32 seconds. Dividing it
by the number of requests gives the average wait per request; the "busy"
time, counted once whatever the depth, is what `%util` uses.
