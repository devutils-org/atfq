---
id: crypto.primitives
kind: concept
title: "Hashes, encryption and signatures"
aliases: [hash, hashing, hash function, sha256, sha-256, sha256sum, md5, md5sum, sha1, encryption, symmetric encryption, aes, aes-gcm, chacha20, signature, digital signature, hmac, checksum, password hashing, bcrypt, argon2]
hubs: []
probes: [crypto.setup]
see_also: [crypto.public-key, crypto.random, hw-cpu.instructions]
commands: [sha256sum, openssl]
paths: [/proc/crypto]
---

## TL;DR

- A hash turns any data into a short fingerprint (SHA-256: 32 bytes).
  Same input, same hash; you can't go back, nor find two inputs with the
  same hash.
- Encryption needs a key to read the data again. With symmetric
  encryption (AES, ChaCha20) one shared key both locks and unlocks.
- A signature proves who wrote something and that it wasn't changed:
  made with a private key, checked with the public one
  ({{crypto.public-key}}).

```
sha256sum ubuntu.iso            # compare with the published hash
sha256sum -c SHA256SUMS --ignore-missing
```

## Pitfalls

- MD5 and SHA-1 are broken against attackers: fine to spot accidental
  corruption, never for security.
- Passwords are stored with slow hashes made for it (yescrypt, bcrypt,
  Argon2), never a plain SHA-256: fast hashes make guessing fast.
- Encryption without authentication lets an attacker alter data
  unnoticed: use modes like AES-GCM or ChaCha20-Poly1305.
- Don't invent your own scheme: use a library's high-level API
  (libsodium, your language's standard crypto).

## Deep dive

### Why is a hash enough to check a download?

If the hash on the official site matches the file's, the file is the
one they published: finding a different file with the same SHA-256 is
out of reach. The hash itself must come from a trusted place (an HTTPS
page or a signed file), otherwise an attacker swaps both.

### How fast is encryption today?

With the CPU's AES instructions (AES-NI on x86, the crypto extensions on
ARM), one core encrypts several gigabytes per second: faster than most
disks and networks. That's why encrypting disks and all web traffic
costs almost nothing now. Chips without them use ChaCha20, fast in plain
software.
