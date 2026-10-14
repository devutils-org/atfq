# ROADMAP — atfq (`idk`)

> A Linux companion for the terminal: short answers first, your machine's real state second, the deep "why" only if you want it.

This file is the single source of truth for building the proof of concept. Read it fully before writing code. When a decision here conflicts with your instinct, follow this file and note the disagreement in `DECISIONS.md`.

**Naming:** the project and the crate are called **atfq** (*Ask The F\*cking Question*, the heritage of the original ATFQ knowledge-base project). The command users type is **`idk`** ("I don't know"), the same way the `ripgrep` package installs the `rg` command. In `Cargo.toml`: package `name = "atfq"`, with a `[[bin]]` section `name = "idk"`. Keep the binary name in one `const BIN_NAME` for help texts. Before release, check that no common distro package already ships an `idk` binary.

---

## 1. Vision

Two problems, one tool:

1. **Commands are hard to remember.** `find`, `awk`, `xargs`, regex: the syntax is dense and man pages are written for people who already know the answer.
2. **The system is opaque.** Which path is for what, which scheduler is running, why RAM looks "full", what a zombie process is, how boot works.

`idk` answers both with **cards** organized like a book (inspired by *The Rust Programming Language*: a flat, numbered table of contents, no deep trees). Every card leads with a TL;DR, then shows what is true **on this machine** (live probes), then pitfalls, then an optional deep dive.

Entry points:

- **Hubs** for a device or domain (`idk disk`, `idk memory`, `idk cpu`): live overview + links to commands, paths and cards.
- **Concept cards** for a precise question (`idk free`, `idk setuid`, `idk find`).

The same core is exposed three ways: CLI, TUI, and an MCP server so AI agents can use it as a read-only knowledge + introspection tool.

## 2. Principles

1. **Answer first.** The first screen solves the problem. Explanations come after, and are hidden by default.
2. **Plain English, not man pages.** See the writing guide (section 9). If a sentence would fit in a man page, rewrite it.
3. **Live over static.** When the machine can answer, show the machine's answer.
4. **Read-only, always.** `idk` never executes the user's commands and never modifies the system. It only shows commands and reads state.
5. **Works offline, needs nothing.** One binary, content embedded, no network, no root, no config required.
6. **Degrade gracefully.** A missing file, tool or permission produces a short note ("needs root", "not available on this system"), never a crash or a stack trace.
7. **One core, many surfaces.** CLI, TUI and MCP consume the same structured data. No logic lives in a surface.

## 3. Non-goals (for the PoC)

- Replacing man pages or covering every flag of every command.
- Running, wrapping or "fixing" commands for the user.
- Calling any AI API or using the network.
- Supporting macOS, BSD or Windows (it may compile there; probes return `Unsupported`).
- Natural language → command translation (later, via AI).
- User accounts, sync, plugins, a website.

## 4. Tech stack

- **Rust**, stable toolchain, edition 2021.
- `clap` (derive) for the CLI.
- `ratatui` + `crossterm` for the TUI.
- `serde`, `serde_yaml`, `serde_json` for frontmatter and structured output.
- A Markdown parser (`pulldown-cmark`) to split card bodies into sections.
- `include_dir` (or a `build.rs` step) to embed `content/` in the binary.
- `insta` for snapshot tests of rendered output.
- MCP: the official Rust MCP SDK (`rmcp`), stdio transport.

Keep dependencies minimal. Justify any new crate in `DECISIONS.md`.

## 5. Architecture

Single crate (library + binary) for the PoC. Split into a workspace only if it becomes painful.

```
atfq/
  Cargo.toml
  ROADMAP.md
  DECISIONS.md            decisions taken during development, one line each
  content/
    SUMMARY.md            table of contents (mdBook-style), defines order and numbering
    hubs/
      memory.md
      disk.md
    cards/
      fs/
      perms/
      proc/
      memory/
      kernel/
      cmd/
  src/
    main.rs               CLI entry
    lib.rs
    content/              loading, parsing, validation, index
    resolve.rs            query → hub | card | suggestions
    probes/               one module per domain, pure functions
    render/               text renderer (CLI), shared by TUI where possible
    tui/
    prompt.rs             "ask AI" prompt builder
    mcp.rs                MCP server
  tests/
    fixtures/             fake /proc and /sys trees
```

### Data flow

```
content/*.md ──parse──▶ Card / Hub (structs)
                              │
query ──resolve──▶ Card/Hub ──┼──▶ probes (by name) ──▶ ProbeResult (structured)
                              │
                              ▼
                     View model (sections + probe data)
                     ├──▶ CLI text renderer
                     ├──▶ TUI widgets
                     ├──▶ JSON (--json, MCP)
                     └──▶ AI prompt builder
```

### Probes

- A probe is a Rust function: `fn(&ProbeCtx) -> ProbeResult`.
- `ProbeCtx` carries a **filesystem root** (`/` in production, a fixture dir in tests). All `/proc` and `/sys` reads go through it. This makes every probe testable without a real machine.
- `ProbeResult` is structured data (serde-serializable) **plus** a status: `Ok`, `Partial`, `NeedsRoot`, `Unavailable`, `Unsupported`. Rendering is never done inside a probe.
- Prefer reading `/proc`, `/sys` and `/etc` directly over calling external binaries.
- If an external binary is unavoidable: allowlist only, absolute resolution via `PATH`, fixed arguments (never user input), 500 ms timeout, missing binary → `Unavailable`.
- Probes are referenced from cards by name, e.g. `mem.summary`, `disk.tree`, `proc.states`. A registry maps names to functions.
- Target distros for manual testing: Debian/Ubuntu, Fedora, Arch.

## 6. Content format

Content is Markdown with YAML frontmatter, one file per card or hub. `content/SUMMARY.md` defines the table of contents and chapter numbers, mdBook-style:

```markdown
# Summary

- [Filesystem](fs)
  - [The hierarchy in 2 minutes](cards/fs/hierarchy.md)
- [Processes & scheduling](proc)
  - [Process states and zombies](cards/proc/states.md)
- [Memory](hubs/memory.md)
  - [free vs available](cards/memory/free-vs-available.md)
- [Finding and transforming](cmd)
  - [find](cards/cmd/find.md)
```

Chapter numbers (`4.1`, `5.1`…) are **derived from SUMMARY.md**, never written by hand in cards.

### 6.1 Concept card

```markdown
---
id: memory.free-vs-available
kind: concept
title: free vs available
aliases: [free, available, ram usage, meminfo, "memory full"]
hubs: [memory]
probes: [mem.summary]
see_also: [memory.oom-killer, kernel.sysctl]
commands: [free, vmstat]
paths: [/proc/meminfo]
---

## TL;DR

- **free**: memory nobody is using at all.
- **available**: what your apps can still get without swapping.
- Look at **available**, not free.

    free -h

## Pitfalls

- "free" near zero is normal: Linux fills spare RAM with cache.
- That cache is handed back as soon as an app needs it.
- A little swap in use is not a problem by itself.

## Deep dive

### Why is unused RAM wasted RAM?
...

### How is "available" calculated?
...

### What is swappiness and when should I change it?
...
```

Rules:

- Required sections, in this order: `## TL;DR`, `## Pitfalls` (may be omitted if truly none), `## Deep dive` (optional).
- The live block ("On your machine") is **not** written in the file. The renderer inserts it after TL;DR when the card has `probes`.
- Each `###` under Deep dive is one collapsible question. Questions follow the ATFQ heritage: *what is it, why does it exist, how does it work, what came before, what are the tradeoffs.*
- `see_also`, `hubs` reference ids. `commands` and `paths` are plain strings.

### 6.2 Hub

```markdown
---
id: disk
kind: hub
title: Disk
aliases: [disks, storage, ssd, hdd, nvme, partitions]
probes: [disk.tree]
commands:
  - { cmd: "lsblk", desc: "tree of disks and partitions" }
  - { cmd: "df -h", desc: "free space per filesystem" }
  - { cmd: "du -sh *", desc: "size of each folder here" }
  - { cmd: "findmnt", desc: "mount points and their options" }
  - { cmd: "iostat -x 1", desc: "live disk activity" }
  - { cmd: "smartctl -a /dev/X", desc: "disk health (needs root)" }
paths:
  - { path: "/dev/disk/by-uuid/", desc: "stable names for disks" }
  - { path: "/sys/block/X/queue/", desc: "scheduler, rotational flag" }
  - { path: "/etc/fstab", desc: "what gets mounted at boot" }
cards: [fs.inodes-links, proc.nice]
---

Everything about storage devices: what's plugged in, how full it is, how it's mounted.
```

Rules:

- A hub has a one or two sentence intro, then the renderer shows: live block, Commands, Paths, Cards.
- A `commands` entry may later gain a `card:` field when that command has its own card (e.g. `find` → `cmd.find`).

### 6.3 Validation (enforced by a test, not by hope)

- Every `id` is unique.
- Every reference (`see_also`, `hubs`, `cards`) resolves.
- Every probe name exists in the registry.
- Every file is listed in SUMMARY.md exactly once, and vice versa.
- Aliases do not collide across cards (a collision between a hub and a card is allowed: the hub wins).
- TL;DR exists and is at most 12 lines.

## 7. Query resolution

`idk <query>` resolves in this order:

1. Exact hub id or hub alias → hub.
2. Exact card id or card alias → card.
3. Chapter number (`5.1`) → card.
4. Fuzzy match on titles and aliases → best match shown, up to 3 "Did you mean" suggestions.
5. Nothing → short message + suggestion to try `idk search` or `idk ask`.

If a word matches both a hub and a card, show the hub and point to the card at the top ("Looking for *free vs available*? → `idk 5.1`").

## 8. Surfaces

### 8.1 CLI

```
idk <query>              TL;DR + live block + pitfalls (deep dive hidden)
idk <query> --why        everything, deep dive expanded
idk <query> --json       structured output (card + probe results)
idk toc                  table of contents
idk search <words>       full-text search, ranked list
idk probe <name>         run one probe, print its result
idk ask <query> [text]   print an AI-ready prompt (phase 3)
idk tui                  interactive mode (phase 2); also when run with no args in a TTY
idk mcp                  run the MCP server on stdio (phase 4)
```

Output rules:

- Max width 80 columns; wrap prose, never wrap commands.
- Colors only on a TTY; respect `NO_COLOR`.
- Commands in TL;DR are visually distinct and copy-pasteable (no prompt characters like `$`).
- Dangerous commands (`-delete`, `rm`, `dd`, `chmod -R`…) are marked with a warning.
- Exit code 0 on success, 1 on no match, 2 on usage error.

### 8.2 TUI

Layout inspired by the TRPL sidebar:

```
┌─ Contents ──────────┬─ 5.1 free vs available ──────────────┐
│ Hubs                │ TL;DR                                 │
│   cpu memory disk   │   ...                                 │
│ 1. Filesystem       │ On your machine ⚡                    │
│ ...                 │   ...                                 │
│ 5. Memory           │ ▸ Why is unused RAM wasted RAM?       │
│ > 5.1 free vs avail │ ▸ How is "available" calculated?      │
├─────────────────────┴───────────────────────────────────────┤
│ j/k move · enter expand · / search · r refresh probes · ? ask│
└─────────────────────────────────────────────────────────────┘
```

- Hubs are listed first, as the "start here" entry.
- Links (`see_also`, hub cards) are navigable.
- `r` reruns probes for the current view.

**Mouse.** The TUI is clickable, on top of a complete keyboard interface (the mouse never does something a key cannot):

- Each widget reports the **regions** it drew (a `Rect` plus an id: a TOC entry, a link, a hub command, a deep dive question). A hit test on these regions is the only mouse logic; no widget handles raw coordinates.
- **Left click** selects what is under it; a click on what is **already selected** goes in, like `Enter` (opens a card, follows a link, expands/collapses a question). No double-click timer: crossterm does not report double clicks, and a timer would make tests depend on time.
- **Right click** goes back, like `Backspace` (navigation keeps a history).
- **Wheel** scrolls the pane under the pointer.
- Clicking a pane gives it focus.
- Mouse capture is **on by default**; `M` toggles it off to give the terminal its own text selection back (to copy a command). The footer shows the current state. Inside tmux it needs `set -g mouse on`.
- Move, drag and button-up events are ignored.

### 8.3 Ask AI (prompt builder)

`idk ask memory "why is my laptop swapping?"` prints a prompt to stdout containing:

1. The user's question.
2. System context: distro, kernel version, architecture.
3. The resolved card or hub TL;DR.
4. The relevant probe results (as compact text).
5. An instruction to answer concisely and in plain English.

No network call. The user pipes it to their clipboard or AI tool. Before the prompt, print to stderr a one-line list of what system data is included. Add `--redact` to strip hostname, username and home paths.

### 8.4 MCP server

Stdio transport, read-only tools:

- `lookup(query)` → resolved card or hub as JSON, with probe results.
- `search(query)` → ranked list of ids and titles.
- `toc()` → table of contents.
- `run_probe(name)` → one probe result.
- `list_probes()` → available probe names and one-line descriptions.

Tool descriptions must make clear that the server never modifies the system.

## 9. Writing guide (content)

**Audience:** a developer comfortable in a terminal, who is not a sysadmin and does not want to read a man page.

Do:

- Write like you're explaining to a colleague: "you", short sentences, active voice.
- Put the most common use first. The rare flag can wait or go.
- Define any jargon the first time it appears, in a few words ("page cache: file data Linux keeps in RAM to avoid rereading the disk").
- Give every command a real, runnable example with realistic arguments.
- Mark dangerous commands and say what to do first ("run it without `-delete` and check the list").
- In Deep dive, answer real questions: why it exists, how it works, what came before, the tradeoffs.
- When behavior depends on version, say so ("since kernel 6.6").

Don't:

- Use man-page notation: no `[-abc] <file>...`, no "SYNOPSIS", no exhaustive flag lists.
- Write "simply", "just", "obviously".
- Paste long command output; show only the lines that matter.
- State facts you have not verified. If unsure, leave it out and add a `TODO(verify)` comment.

Size limits: TL;DR ≤ 12 lines, Pitfalls ≤ 6 bullets, each Deep dive question ≤ ~150 words.

## 10. Phases

Each phase ends with every acceptance criterion passing. Do not start a phase before the previous one is done. Commit at the end of each phase.

### Phase 0 — Skeleton and content pipeline

Deliverables:

- Crate setup, CLI skeleton with `idk <query>`, `--why`, `idk toc`.
- Content parsing (frontmatter + sections), SUMMARY.md parsing, chapter numbering, embedding in the binary.
- Validation test (section 6.3).
- Resolution steps 1–3 (exact matches and chapter numbers).
- Text renderer, without the live block (placeholder "probes: not implemented yet").
- Initial content, written per section 9:
  - Hubs: `memory`, `disk`.
  - Cards: `proc.states` (process states and zombies), `memory.free-vs-available`, `cmd.find`.

Acceptance:

- `cargo test` passes, including validation.
- `idk memory`, `idk disk`, `idk free`, `idk zombie`, `idk find`, `idk 5.1` each show the right content.
- `idk free --why` shows deep dive questions; `idk free` does not.
- `idk toc` matches SUMMARY.md with correct numbers.
- `cargo clippy -- -D warnings` is clean.

### Phase 1 — Live probes and search

Deliverables:

- Probe framework: `ProbeCtx` with root path, `ProbeResult` with status, registry.
- Probes: `mem.summary` (from `/proc/meminfo` + swappiness), `disk.tree` (from `/sys/block`, `/proc/self/mountinfo`, `statvfs`), `proc.states` (count of processes per state from `/proc/*/stat`).
- Live block in the renderer for cards and hubs.
- `idk probe <name>`, `--json`.
- Fuzzy matching (resolution step 4) and `idk search`.

Acceptance:

- Each probe has unit tests against fixture trees in `tests/fixtures/`, including a missing-file case.
- Snapshot tests (`insta`) for rendered output of each hub and card, using fixtures.
- Running on a real Linux machine shows plausible values; no panic when a file is unreadable.
- `idk memroy` suggests `memory`.

### Phase 2 — TUI

Deliverables: layout from section 8.2, navigation with history, collapsible deep dive, link following, search, probe refresh, mouse support (regions, click, right click, wheel, `M` toggle).

Acceptance:

- Usable entirely with the keyboard; `q` and `Esc` exit cleanly and restore the terminal (mouse capture included), including after a panic.
- Usable with the mouse: a TOC entry, a link and a deep dive question can each be opened by clicking; right click goes back; the wheel scrolls.
- Snapshot tests of the TUI after a scripted click sequence (events fed directly, no real terminal).
- Works in an 80×24 terminal.
- No logic duplicated from the CLI renderer beyond widget layout.

### Phase 3 — Ask AI prompt

Deliverables: `idk ask` per section 8.3, `--redact`, `?` key in the TUI that shows the prompt.

Acceptance:

- Snapshot test of a generated prompt from fixtures.
- With `--redact`, no hostname, username or `/home/<user>` appears in the output.

### Phase 4 — MCP server

Deliverables: `idk mcp` with the tools from section 8.4.

Acceptance:

- Works with an MCP client (e.g. Claude Desktop or the MCP inspector): each tool returns valid JSON.
- An integration test drives the server over stdio for at least `lookup` and `run_probe`.

### Phase 5 — Content expansion

Add hubs `cpu`, `network`, `boot` and cards chapter by chapter (see section 11). Each new card must pass validation and include a probe when the machine can answer the question.

## 11. Content backlog (after phase 0)

Hubs: cpu, network, boot, processes, permissions.

Cards, by chapter:

- **Filesystem:** the hierarchy in 2 minutes; /tmp vs /var/tmp; /proc and /sys; links, inodes and "disk full"; fstab and mount options; /dev and udev.
- **Permissions:** rwx, octal and umask; setuid, setgid and sticky bit; ACLs and capabilities; sudo and groups; SELinux and AppArmor.
- **Processes & scheduling:** nice, priorities and the scheduler; signals; cgroups.
- **Memory:** the OOM killer; swap and swappiness; page cache.
- **Boot:** from firmware to init; GRUB vs systemd-boot vs UKI; kernel command line; systemd targets.
- **Kernel:** sysctl; modules; dmesg; syscalls and strace.
- **Libraries & binaries:** dynamic linking and ldd; ELF and "command not found"; PATH and command resolution.
- **Environment & shell:** where env variables come from; shell startup files; redirections and file descriptors; exit codes.
- **systemd:** units and timers; why a service won't start; unit files and overrides.
- **Network:** interfaces and routes; DNS resolution; who listens on which port; nftables vs iptables vs ufw.
- **Packages:** which package owns a file; native packages vs Snap/Flatpak.
- **Commands:** grep and regex; awk; sed; xargs; tar; rsync; jq; sort/uniq/cut.

## 12. Working agreement for the agent

- Read this file before each phase.
- Record any decision not covered here in `DECISIONS.md` (date, decision, reason).
- Never weaken a principle in section 2 to make something easier. Ask instead.
- Prefer small, reviewable commits with clear messages.
- When writing content, verify technical facts. A wrong fact is worse than a missing one.
