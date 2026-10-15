# atfq

A Linux companion for the terminal: short answers first, your machine's real
state second, the deep "why" only if you want it.

```
$ atfq free
17.1  free vs available

TL;DR
  - free: memory nobody is using at all, not even as cache.
  - available: what your apps can still get without swapping.
  ...

On your machine
  RAM       10.5 GiB available of 15.5 GiB (68%) · 1.1 GiB completely free
  Cache     7.5 GiB in buff/cache, 604.0 MiB of it shared (tmpfs)
  Swap      256.0 MiB used of 8.0 GiB · swappiness 60
...
```

`atfq` is read-only: it never runs your commands, never changes the system,
never uses the network. One binary, content embedded, no root needed.

## Install

A prebuilt binary, for x86-64 or ARM64 Linux (glibc 2.35 or later):

```
curl -LO https://github.com/moha-bekh/atfq/releases/latest/download/atfq-x86_64-linux.tar.gz
tar xzf atfq-x86_64-linux.tar.gz
install -m 755 atfq-x86_64-linux/atfq ~/.local/bin/
```

Replace `x86_64` with `aarch64` on ARM. Each release lists SHA-256 sums.

From source, with a Rust toolchain:

```
cargo install --path .
```

*atfq* stands for "Ask The F\*cking Question": the project grew out of a
knowledge base of that name.

## Use

```
atfq memory              # a hub: live overview, commands, paths, cards
atfq zombie              # a card, by name or alias
atfq 17.1                # a card, by chapter number
atfq free --why          # with the deep dive
atfq toc                 # table of contents
atfq search swap         # full-text search
atfq probe               # list the live probes; atfq probe disk.tree runs one
atfq free --json         # structured output, for scripts
atfq tui                 # interactive mode (also: atfq, alone, in a terminal)
atfq ask memory "why is my laptop swapping?"   # a prompt to paste into an AI
atfq ask disk --redact   # without hostname, username and home paths
atfq mcp                 # MCP server on stdio, for AI agents
```

Typos are forgiven (`atfq memroy` shows Memory), and error messages work as
queries: `atfq "command not found"`, `atfq "exec format error"`, `atfq degraded`.

Exit codes: 0 found, 1 no match, 2 usage error, 3 internal error.

### Interactive mode

Contents on the left, the page on the right.

| Key | Does |
|---|---|
| ↑ ↓ / j k | move in the contents, scroll the page |
| ← → / h l | switch pane |
| Tab, Shift-Tab | next, previous link or deep dive question |
| Enter | open the link, expand or collapse the question |
| Backspace | back |
| `/` | search |
| `?` | the AI prompt for this page |
| `r` | run the probes again |
| `M` | mouse on or off (off: select text to copy) |
| `q`, Esc | quit (Esc first closes search or the prompt) |

With the mouse: click selects, a second click opens, right click goes back,
the wheel scrolls the pane under the pointer. In tmux, mouse support needs
`set -g mouse on`.

### MCP server

`atfq mcp` serves five read-only tools over stdio: `lookup`, `search`, `toc`,
`run_probe` and `list_probes`. Every answer is JSON.

Claude Code:

```
claude mcp add atfq -- atfq mcp
```

Claude Desktop, in `claude_desktop_config.json` (use the full path of the
binary, as `which atfq` prints it):

```json
{
  "mcpServers": {
    "atfq": { "command": "/home/you/.cargo/bin/atfq", "args": ["mcp"] }
  }
}
```

## What's inside

- 9 hubs (Disk, Permissions, Processes & scheduling, Memory, CPU, Network,
  Boot, GPU, Logs) and 121 cards in 30 chapters, in three parts (how it
  works, your system, shell and commands): `atfq toc` lists them.
- 49 live probes. They read `/proc`, `/sys` and `/etc` directly, plus a
  few files in your home folder: names and public keys in `~/.ssh`, and
  git's config, of which only your name's and email's presence are
  shown. Never a private key or a credential. The only
  external program is `systemctl`, with fixed arguments and a 500 ms
  timeout. A probe that can't answer says why ("needs root", "not
  available on this system") instead of failing.
- The environment can hold secrets: probes only read a short list of
  well-known variables.

## Showing it to someone

`docs/demo.md` is a ten-minute demo script: what to type, what to say, and
what to check each answer against.

## Development

```
cargo test                       # unit, fixture, snapshot and CLI tests
cargo clippy --all-targets -- -D warnings
INSTA_UPDATE=always cargo test   # accept changed snapshots (review the diff!)
```

- `ROADMAP.md` is the plan and the rules: architecture, content format,
  writing guide.
- `DECISIONS.md` records every decision the roadmap didn't cover.
- Content lives in `content/` as Markdown with YAML frontmatter;
  `content/SUMMARY.md` sets the order and the chapter numbers. A test checks
  every rule of the format (unique ids, references, probe names, TL;DR
  length…).
- Probes read through a `ProbeCtx` whose root is `/` in production and a
  folder under `tests/fixtures/` in tests.

## Contributing

Found a mistake in a card? That matters most in a tool people learn from:
open an issue with the card number and what's wrong, ideally with a
source. `CONTRIBUTING.md` explains how to add or fix a card, a probe or
code.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. This covers the code and the content (the cards in
`content/`).

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
