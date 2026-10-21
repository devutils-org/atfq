# Security policy

atfq is read-only: it never runs your commands, never writes to the system and
never uses the network. A way to make it do any of those is a vulnerability.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/devutils-org/atfq/security/advisories/new)
with what you ran, what happened, and the version (`atfq --version`).
You will get an answer within a week.

## Supported versions

Only the latest release gets fixes.

## Verifying a release

Release archives from 0.1.1 on carry a build provenance attestation:

```sh
gh attestation verify atfq-x86_64-linux.tar.gz --repo devutils-org/atfq
```
