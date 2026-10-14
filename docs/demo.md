# Giving the demo: ten minutes, six acts

A script for showing `idk` to someone: what to type, what to say, which line
of the output to point at, and what to check it against. Every command here
was run on the aarch64 OrbStack machine (Arch Linux ARM); where a physical
machine shows something else, the difference is said.

The thread through all of it is one sentence: **`idk` answers first, then
shows what is true on this machine, and every live line can be checked with
the command the card itself gives you.**

---

## Before: five minutes of preparation

```console
cargo install --path .        # installs the idk command in ~/.cargo/bin
cargo test                    # the dress rehearsal: every test passes
systemctl --failed            # decides act 2: failed units, or Plan B
npx -y @modelcontextprotocol/inspector --cli idk mcp --method tools/list > /dev/null
                              # downloads the MCP inspector once, for act 6
```

- A terminal of at least 80 x 24; everything below fits in 80 columns.
- **Make a zombie just before act 2**, in the shell you demo from:

  ```console
  (sleep 1 & exec sleep 300) &
  ```

  The subshell starts `sleep 1` in the background, then becomes `sleep 300`,
  which never waits for its child: one second later the child is a zombie,
  for five minutes. Run it again if the demo runs late.
- In tmux, mouse support needs `set -g mouse on` (act 5).
- No network is needed, except for the inspector's first download.

---

## Act 1 -- the answer first (1 min)

```console
idk free
free -h
```

**Say:** "Someone sees 'free: 1.0 GiB' on a 4 GiB machine and thinks the RAM
is full. The first screen answers that, in three lines, before any
explanation."

**Point at:** the three TL;DR bullets and the command under them, then the
`On your machine` block: `RAM  2.6 GiB available of 3.9 GiB (68%) · 1.0 GiB
completely free`. Check it against the `available` and `free` columns of
`free -h`: the same 2.6 GiB and 1.0 GiB. Last, the line `Deep dive: 3
questions. Read them with idk 5.1 --why`: the why exists, but it waits until
you ask.

---

## Act 2 -- what is true on this machine (2 min)

The zombie made during the preparation is the answer key: you know it's
there, and `ps` can confirm it.

```console
idk zombie
ps -eo pid,ppid,stat,comm | awk '$3 ~ /^Z/'
```

**Say:** "The card explains zombies, then looks for them here. And the
command to check it is in the card itself."

**Point at:** `Zombies  PID 639480 sleep, parent 639479 sleep`, then the `ps`
line: the same two PIDs, state `Z`. The TL;DR already said what to do: "You
can't kill a zombie: fix or restart its parent." Here, the parent is `sleep
300`.

**Then the system's own state:**

```console
idk degraded
systemctl --failed
```

**Point at:** `State  degraded: some units failed` and the failed units, the
same as `systemctl --failed` lists (on this VM: `console-getty.service` and
`systemd-resolved-varlink.socket`). `idk` read it through `systemctl`, the
only external program it ever runs: fixed arguments, 500 ms at most.

**Plan B (nothing failed):** the block says `State  running: every unit is
fine` and `Failed  none`. Show the disks instead: `idk disk`, and check the
free space of `/` against `df -h /`.

---

## Act 3 -- type what you saw (1.5 min)

```console
idk "command not found"
idk "exec format error"
idk "Address already in use"
idk "No space left on device"
idk memroy
idk overcomit; echo $?
```

**Say:** "You don't need to know what a thing is called. Paste the error
message you got: that's the query."

**Point at:** each first line: `12.2 ELF and "command not found"` twice,
`8.1 Who listens on which port`, `1.4 Links, inodes and "disk full"`. Then
the typo: `No exact match for "memroy", showing "Memory".` And when nothing
matches, it doesn't guess: `Nothing matches "overcomit".` with `Related: idk
5.2 (The OOM killer), idk 10.1 (sysctl)`, found in the text of the cards, and
exit code 1 for scripts.

---

## Act 4 -- a book, not a pile of pages (1 min)

```console
idk toc
idk 5.1 --why
```

**Say:** "It's organized like a book: hubs to start from, then numbered
chapters. A number is an address you can type."

**Point at:** the hubs first (`idk disk`, `idk memory`…), then the chapters;
in `--why`, the three questions marked `▾`, each answering why it exists,
how it works, or what to change. Numbers never move: new cards go at the end
of their chapter.

---

## Act 5 -- the picture (2 min)

```console
idk
```

**Say:** "The same pages, to browse." With no argument in a terminal, `idk`
opens the interactive mode.

Then, in this order:

1. Click `4.1 Process states and zombies` in the contents, then click it
   again: the first click selects, the second opens. The zombie is in the
   live block.
2. Scroll down with the wheel to a question `▸`, click it twice: the answer
   opens and moves to the top.
3. Right click: back.
4. `/` then `zomb`, Enter: search.
5. `?`: the prompt for an AI assistant, about this page. Esc closes it.
6. `q`: the terminal comes back as it was.

**Fallback:** everything works with the keyboard alone (↑↓, Tab to go from
link to link, Enter, Backspace); the footer lists the keys. If the mouse
does nothing in tmux, `set -g mouse on` is missing; `M` turns capture off to
select text.

---

## Act 6 -- for an AI, without the network (2 min)

```console
idk ask memory "why is my laptop swapping?" --redact
```

**Say:** "`idk` never calls an AI. It writes the question an AI needs: your
words, the system, the card, and what this machine shows. You paste it where
you want."

**Point at:** the line on stderr, before the prompt: `Included: distribution,
kernel version, architecture, live data (mem.summary). Redacted: hostname,
username, home paths.` Then in the prompt, `## What my machine shows right
now` with the live block. With `--redact`, the username appears nowhere.

**Then an agent asks the same machine:**

```console
npx -y @modelcontextprotocol/inspector --cli idk mcp --method tools/list
npx -y @modelcontextprotocol/inspector --cli idk mcp --method tools/call \
    --tool-name run_probe --tool-arg name=proc.states
```

**Point at:** five tools, each with `"readOnlyHint": true` and "Read-only" in
its description; then the same zombie as in act 2, now as JSON:
`"zombies":[{"pid":639480,"comm":"sleep","ppid":639479,"parent":"sleep"}]`.
One fact, four surfaces: the CLI, the TUI, the prompt and an agent.

---

## If there is time

| Show | Command | Point at |
|---|---|---|
| a messy PATH | `idk path` | each folder once, in search order; `Repeated` for the doubles and `(doesn't exist)` for dead entries (27 entries on this VM, 13 of them repeats) |
| the disks | `idk disk` | the btrfs root matched to `vdb1` by its source, free space as `df` counts it; on a laptop, the NVMe model and `SSD` |
| raw data for scripts | `idk probe mem.summary --json \| jq .data.available` | bytes, the same figure as the live block |
| search | `idk search delete matches` | `idk 6.1 find`, with the line that matched |
| boot | `idk boot` | on this VM `device tree (no UEFI)`; on a laptop `UEFI · Secure Boot on` and the bootloader |
| dangerous commands | `idk find` | under `find . -name '*.tmp' -delete`, the line `⚠ changes or deletes data: check what it targets first`; a pitfall says to run it without `-delete` first and check the list |

---

## Questions people ask

- **"How is this different from `man`, `tldr` or a search engine?"** `man`
  lists every option for people who already know the answer; `tldr` gives
  examples. `idk` gives the answer first, then this machine's state, then
  the why, and it works offline.
- **"Can it break something?"** No. It reads files in `/proc`, `/sys` and
  `/etc`, and runs one program, `systemctl`, with fixed arguments. It never
  runs what you type.
- **"Does it need root?"** No. Where root would show more (another user's
  process on a port, AppArmor profiles), it says so: `needs root`, with the
  command that would.
- **"What does it send to an AI?"** Nothing: `idk ask` only prints. The
  environment is never dumped, only a short list of well-known variables,
  because environments hold tokens.
- **"Which distributions?"** Debian, Ubuntu, Fedora and Arch are the
  targets; the probes were checked against the files of Debian 13, Ubuntu
  24.04 and Fedora 44, and run live on Arch ARM. It compiles for x86-64 and
  32-bit ARM.
- **"How do I add a card?"** A Markdown file in `content/`, a line in
  `content/SUMMARY.md`, and `cargo test`: a test checks ids, references,
  probes and the TL;DR length. `ROADMAP.md` section 9 is the writing guide.
