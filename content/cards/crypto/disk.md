---
id: crypto.disk
kind: concept
title: "Disk encryption: LUKS and the TPM"
aliases: [luks, luks2, cryptsetup, disk encryption, full disk encryption, encrypted disk, dm-crypt, tpm, tpm2, systemd-cryptenroll, recovery key, crypttab]
hubs: [disk]
probes: [crypto.setup, disk.tree]
see_also: [crypto.primitives, boot.sequence, hw-storage.flash]
commands: [cryptsetup, systemd-cryptenroll, lsblk]
paths: [/etc/crypttab]
---

## TL;DR

- LUKS encrypts a whole partition: without the key, a stolen disk or
  laptop gives nothing. Once unlocked, files are readable as usual.
- Your passphrase doesn't encrypt the data directly: it unlocks a random
  master key stored in the LUKS header, in one of several key slots.
- A TPM can hold an unlock key and release it only if the boot chain is
  unchanged: no passphrase at boot, still protected if the disk is
  removed.

```
lsblk -f                          # crypto_LUKS partitions
sudo cryptsetup luksDump /dev/nvme0n1p3     # key slots, cipher
sudo systemd-cryptenroll --tpm2-device=auto /dev/nvme0n1p3
```

## Pitfalls

- Damage to the LUKS header loses everything: keep a copy elsewhere
  (`cryptsetup luksHeaderBackup`).
- Keep a recovery passphrase or key: a firmware update or Secure Boot
  change makes the TPM refuse, and asks for it.
- Encryption protects a machine that's off. While it runs, files are as
  readable as on any disk.
- Changing a passphrase is instant (another key slot); changing the
  master key means re-encrypting the disk.

## Deep dive

### How does the TPM know the boot is the same?

As each stage boots, it measures the next (firmware, bootloader, kernel,
Secure Boot settings) into the TPM's registers (PCRs). The disk key is
sealed to chosen register values: the TPM releases it only if they
match. Boot something else, or move the disk to another machine, and the
key stays locked.

### Does encryption slow the disk down?

Barely, with AES instructions in the CPU: encryption runs at gigabytes
per second per core. On some SSDs, `discard` must be allowed in
`/etc/crypttab` for TRIM to reach the drive ({{hw-storage.flash}}).
