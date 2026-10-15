---
id: crypto.certificates
kind: concept
title: "Certificates: how HTTPS proves who it is"
aliases: [certificate, certificates, x509, x.509, ca, certificate authority, ca certificates, ca bundle, self-signed, self-signed certificate, let's encrypt, certificate expired, update-ca-certificates, openssl s_client, ssl certificate, unable to get local issuer certificate, certificate verify failed]
hubs: []
probes: [crypto.setup]
see_also: [netmodel.website, crypto.public-key, time.ntp]
commands: [openssl, curl, update-ca-certificates, update-ca-trust]
paths: [/etc/ssl/certs/, /etc/pki/tls/certs/, /usr/local/share/ca-certificates/]
---

## TL;DR

- A certificate says "this public key belongs to example.com", signed by
  a certificate authority (CA). Your system trusts 100 to 150 root CAs.
- During the TLS handshake the server sends its certificate and the
  intermediates; your client checks the chain up to a trusted root, the
  name, and the dates.
- Let's Encrypt gives free certificates that renew automatically; their
  lifetime is short, and getting shorter across the industry.

```
openssl s_client -connect example.com:443 </dev/null   # chain, dates
openssl x509 -in cert.pem -noout -subject -issuer -dates
```

## Pitfalls

- "unable to get local issuer certificate": the server doesn't send its
  intermediate certificate, or the CA is a private one you must add.
- Adding a company CA: copy it to `/usr/local/share/ca-certificates/`
  and run `sudo update-ca-certificates` (Debian, Ubuntu), or
  `/etc/pki/ca-trust/source/anchors/` and `update-ca-trust` (Fedora).
- Python's requests and Node.js often use their own CA list, not the
  system's: `REQUESTS_CA_BUNDLE`, `NODE_EXTRA_CA_CERTS`.
- `curl -k` turns all checking off: it hides the problem and invites a
  man in the middle.

## Deep dive

### Why trust a chain?

Root CAs keep their keys offline and sign a few intermediate CAs, which
sign servers' certificates. Your client only stores the roots; each
link's signature proves the next was approved by the one above. Every
certificate issued is also logged publicly (Certificate Transparency),
so a CA issuing one for your domain behind your back gets noticed.

### What does a CA check before signing?

That the requester controls the domain: by serving a token on the site
over HTTP, or creating a DNS record (the ACME protocol automates it).
Basic certificates prove nothing more, not who runs the site, just that
you're talking to whoever controls the name.
