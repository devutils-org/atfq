---
id: crypto.public-key
kind: concept
title: Public-key cryptography
aliases: [public key, public key cryptography, private key, asymmetric, asymmetric encryption, rsa, ed25519, ecdsa, elliptic curve, diffie-hellman, key exchange, forward secrecy, gpg, pgp, gnupg, post-quantum, ml-kem]
hubs: []
probes: [ssh.setup]
see_also: [crypto.primitives, crypto.certificates, remote.ssh-keys]
commands: [ssh-keygen, gpg]
paths: [~/.ssh/, ~/.gnupg/]
---

## TL;DR

- A key pair: the private key stays secret, the public one can go
  anywhere. What one does, only the other can check or undo.
- Signatures: sign with the private key, anyone verifies with the public
  one. That's how SSH logins, package updates and certificates work.
- Key exchange: two strangers agree on a shared secret over an open
  network, then switch to fast symmetric encryption.

```
ssh-keygen -t ed25519            # a key pair for SSH
gpg --verify file.sig file       # check a signed download
```

## Pitfalls

- Whoever has your private key is you: protect it with a passphrase, and
  never copy it to servers.
- RSA keys under 2048 bits are too weak today; Ed25519 is shorter, faster
  and safer to use.
- A public key alone proves nothing about who owns it: you need a trusted
  way to get it (a certificate, a fingerprint checked in person).

## Deep dive

### What makes it work?

Problems easy one way and hard the other. RSA rests on factoring: easy to
multiply two large primes, infeasible to find them from the product.
Elliptic curves (Ed25519, X25519) rest on a similar one-way operation on
curve points, with much smaller keys for the same strength.

### What do quantum computers change?

A large enough quantum computer would break RSA and elliptic curves.
Data recorded today could be decrypted later, so key exchanges are
moving first: OpenSSH 10 and current browsers combine X25519 with ML-KEM,
a post-quantum algorithm, so both would have to fall. Symmetric
encryption and hashes only need longer keys.
