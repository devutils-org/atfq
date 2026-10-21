<p align="center"><code>devutils / EXPLAIN</code></p>

<h1 align="center">ATFQ</h1>

<p align="center">A Linux companion for the terminal.</p>

<p align="center">
  <a href="https://devutils-org.github.io/tools/atfq/">Website</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="https://github.com/devutils-org">devutils</a>
</p>

## Overview

- Get a short answer, your machine's live state, then the deeper explanation.
- Browse offline in the terminal, query from scripts or connect an MCP client.
- Read-only: no command execution, system changes or network access; no root needed.

## Install

A static binary for Linux x86-64: it needs no particular libc, so it runs on any
distribution, Alpine included ([checked with distroproof](#checked-on-every-distribution)).
Use `aarch64` instead of `x86_64` on ARM64. Each release archive has a signed build provenance attestation, verified using `gh attestation verify`.

```sh
curl -fL https://github.com/devutils-org/atfq/releases/latest/download/atfq-x86_64-linux.tar.gz \
  -o atfq-x86_64-linux.tar.gz && \
gh attestation verify atfq-x86_64-linux.tar.gz --repo devutils-org/atfq && \
tar xzf atfq-x86_64-linux.tar.gz && \
mkdir -p ~/.local/bin && \
install -m 755 atfq-x86_64-linux/atfq ~/.local/bin/
```

Keep `~/.local/bin` on your `PATH`. From a source checkout: `cargo install --path .`.

## Use

```sh
atfq memory          # live overview and related explanations
atfq free --why      # short answer and deep dive
atfq search swap     # search the embedded content
atfq tui             # interactive browser
atfq mcp             # read-only MCP server on stdio
```

## Checked on every distribution

[distroproof](https://github.com/devutils-org/distroproof) boots each distribution
it knows on its own kernel and runs atfq there: the static build and the latest
release side by side, every hub, search, the AI prompt, the MCP server and every
probe, which must answer without crashing whatever the machine lacks.

```sh
sh scripts/distroproof/build.sh
distroproof up --no-tui --jobs 4 --json dist/matrix.json
python3 scripts/distroproof/summary.py dist/matrix.json
```

Latest run (2026-10-21, 0.1.1): the static build passes every check on all 19
targets — Debian 10 to sid, Ubuntu 18.04 to 24.04, Rocky 8 and 9, Fedora 43 and
44, Arch, Alpine, NixOS, Gentoo, Debian as root and Fedora under lockdown. The
probes a machine cannot answer say so: no CPU frequency in a VM, no systemd on
Alpine. The 0.1.0 release, linked against glibc 2.34, did not start on seven of
them: Debian 10 and 11, Ubuntu 18.04 and 20.04, Rocky 8, Alpine and NixOS.

## Documentation

[CLI, TUI and MCP reference](REFERENCE.md) · [Demo](docs/demo.md) ·
[Content index](content/SUMMARY.md) · [Roadmap](docs/ROADMAP.md) ·
[Decisions](docs/DECISIONS.md) · [Changelog](CHANGELOG.md) · [Security](SECURITY.md)

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

See [Contributing](CONTRIBUTING.md) for cards, probes and code.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Applies to code and content; see [contribution terms](REFERENCE.md#license).
