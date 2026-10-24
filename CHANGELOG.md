# Changelog

## 0.1.2 — 2026-10-24

- TUI sidebar section and chapter titles use the accent colour for readability.
- Resize the sidebar with `+` / `-` or by dragging its right border with the mouse.
  The width adapts to smaller terminals and is restored when space is available.
- Show the sidebar width shortcuts in the bottom help bar.

## 0.1.1 — 2026-10-21

- Static release binaries (musl) for x86_64 and aarch64: no glibc requirement.
  0.1.0's binaries needed glibc 2.34 and did not start on Debian 10 and 11,
  Ubuntu 18.04 and 20.04, Rocky 8, Alpine or NixOS.
- Each release archive is signed by a build provenance attestation
  (`gh attestation verify`).
- `distroproof.toml` runs the static build and the latest release side by side
  in VMs of every distribution distroproof knows. The static build passes every
  check on all 19 targets (2026-10-21).
- New card and probe `kernel.tracing`: why a tool may not watch the kernel.

## 0.1.0 — 2026-10-18

First release.

- Hubs for memory, disk, CPU, GPU, network, boot, processes, permissions and logs,
  and over a hundred cards, numbered like a book (`atfq toc`).
- Live probes read the machine (procfs, sysfs, a few read-only commands), with a
  short note instead of a crash when something is missing.
- Fuzzy matching and full-text search; `--why` for the deep dive; `--json`.
- A TUI with keyboard and mouse, `atfq ask` to build a prompt for an AI assistant
  (`--redact`), and `atfq mcp`, a read-only MCP server on stdio.
