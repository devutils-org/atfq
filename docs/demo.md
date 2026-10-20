# Giving the demo: ten minutes, six acts

A script for showing `atfq` to someone: what to type, what to say, which line
of the output to point at, and what to check it against. Every command here
was run on the aarch64 OrbStack machine (Arch Linux ARM); where a physical
machine shows something else, the difference is said.

The thread through all of it is one sentence: **`atfq` answers first, then
shows what is true on this machine, and every live line can be checked with
the command the card itself gives you.**

---

## Before: five minutes of preparation

```console
cargo install --path .        # installs the atfq command in ~/.cargo/bin
cargo test                    # the dress rehearsal: every test passes
systemctl --failed            # decides act 2: failed units, or Plan B
npx -y @modelcontextprotocol/inspector --cli atfq mcp --method tools/list > /dev/null
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
atfq free
free -h
```

**Say:** "Someone sees 'free: 1.0 GiB' on a 4 GiB machine and thinks the RAM
is full. The first screen answers that, in three lines, before any
explanation."

**Point at:** the three TL;DR bullets and the command under them, then the
`On your machine` block: `RAM  2.6 GiB available of 3.9 GiB (68%) · 1.0 GiB
completely free`. Check it against the `available` and `free` columns of
`free -h`: the same 2.6 GiB and 1.0 GiB. Last, the line `Deep dive: 3
questions. Read them with atfq 17.1 --why`: the why exists, but it waits until
you ask.

---

## Act 2 -- what is true on this machine (2 min)

The zombie made during the preparation is the answer key: you know it's
there, and `ps` can confirm it.

```console
atfq zombie
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
atfq degraded
systemctl --failed
```

**Point at:** `State  degraded: some units failed` and the failed units, the
same as `systemctl --failed` lists (on this VM: `console-getty.service` and
`systemd-resolved-varlink.socket`). `atfq` read it through `systemctl`, the
only external program it ever runs: fixed arguments, 500 ms at most.

**Plan B (nothing failed):** the block says `State  running: every unit is
fine` and `Failed  none`. Show the disks instead: `atfq disk`, and check the
free space of `/` against `df -h /`.

---

## Act 3 -- type what you saw (1.5 min)

```console
atfq "command not found"
atfq "exec format error"
atfq "Address already in use"
atfq "No space left on device"
atfq memroy
atfq overcomit; echo $?
```

**Say:** "You don't need to know what a thing is called. Paste the error
message you got: that's the query."

**Point at:** each first line: `23.2 ELF and "command not found"` twice,
`19.1 Who listens on which port`, `13.4 Links, inodes and "disk full"`. Then
the typo: `No exact match for "memroy", showing "Memory".` And when nothing
matches, it doesn't guess: `Nothing matches "overcomit".` with `Related: atfq
17.2 (The OOM killer), atfq 21.1 (sysctl)`, found in the text of the cards, and
exit code 1 for scripts.

---

## Act 4 -- a book, not a pile of pages (1.5 min)

```console
atfq toc
atfq 17.1 --why
```

**Say:** "It's organized like a book: hubs to start from, then three parts.
How it works, the theory; Your system, the practice; Shell and commands. A
number is an address you can type."

**Point at:** the nine hubs first (`atfq disk`, `atfq memory`… `atfq logs`),
then `Part I · How it works`, `Part II · Your system`, `Part III · Shell and
commands`; in `--why`, the three questions marked `▾`, each answering why it
exists, how it works, or what to change. Cards cite each other by id, so
numbers in the text always match the table of contents.

**Then the theory, on this machine:**

```console
atfq docker
atfq "cpu cache"
```

**Say:** "Even the theory looks at your machine. You thought this was a
VM?"

**Point at:** in `7.1 What a container really is`, `Here  this is a
container: lxc (from /run/systemd/container), inside a virtual machine:
orbstack,virt`: OrbStack runs each "machine" as a container inside one VM,
and the card finds both layers. Then in `2.3 CPU caches`, this chip's caches
(`L2  2 × 12.0 MiB, each shared by 4 CPUs`, `Line  128 bytes`), next to the
pitfall on reading arrays in order. On a typical x86 laptop the line says
64 bytes and the first answer `no container or virtual machine detected`.

---

## Act 5 -- the picture (2 min)

```console
atfq
```

**Say:** "The same pages, to browse." With no argument in a terminal, `atfq`
opens the interactive mode.

Then, in this order:

1. Click `16.1 Process states and zombies` in the contents, then click it
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
atfq ask memory "why is my laptop swapping?" --redact
```

**Say:** "`atfq` never calls an AI. It writes the question an AI needs: your
words, the system, the card, and what this machine shows. You paste it where
you want."

**Point at:** the line on stderr, before the prompt: `Included: distribution,
kernel version, architecture, live data (mem.summary). Redacted: hostname,
username, home paths.` Then in the prompt, `## What my machine shows right
now` with the live block. With `--redact`, the username appears nowhere.

**Then an agent asks the same machine:**

```console
npx -y @modelcontextprotocol/inspector --cli atfq mcp --method tools/list
npx -y @modelcontextprotocol/inspector --cli atfq mcp --method tools/call \
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
| a messy PATH | `atfq path` | each folder once, in search order; `Repeated` for the doubles and `(doesn't exist)` for dead entries (27 entries on this VM, 13 of them repeats) |
| the disks | `atfq disk` | the btrfs root matched to `vdb1` by its source, free space as `df` counts it; on a laptop, the NVMe model and `SSD` |
| raw data for scripts | `atfq probe mem.summary --json \| jq .data.available` | bytes, the same figure as the live block |
| search | `atfq search delete matches` | `atfq 28.1 find`, with the line that matched |
| boot | `atfq boot` | on this VM `device tree (no UEFI)`; on a laptop `UEFI · Secure Boot on` and the bootloader |
| dangerous commands | `atfq find` | under `find . -name '*.tmp' -delete`, the line `⚠ changes or deletes data: check what it targets first`; a pitfall says to run it without `-delete` first and check the list |
| git, too | `atfq "git undo"` | `git restore FILE` marked `⚠`, `git restore --staged FILE` not: one drops your edits, the other only unstages |
| logs | `atfq logs` | the journal `on disk (/var/log/journal): kept across reboots`, its size, and the biggest files in `/var/log` |

---

## Questions people ask

- **"How is this different from `man`, `tldr` or a search engine?"** `man`
  lists every option for people who already know the answer; `tldr` gives
  examples. `atfq` gives the answer first, then this machine's state, then
  the why, and it works offline.
- **"Can it break something?"** No. It reads files in `/proc`, `/sys` and
  `/etc`, plus a few in your home folder (file names and public keys in
  `~/.ssh`, git's config), and runs one program, `systemctl`, with fixed
  arguments. It never opens a private key or a credential, and never runs
  what you type.
- **"What does it cover?"** 122 cards in 30 chapters. The theory: kernel,
  CPU, memory, storage, GPU, networks, containers, concurrency,
  filesystems, time, cryptography, text. The practice: nine hubs from Disk
  to Logs. The shell: environment, find and friends, SSH, git. 50 probes
  answer from the machine.
- **"Does it need root?"** No. Where root would show more (another user's
  process on a port, AppArmor profiles), it says so: `needs root`, with the
  command that would.
- **"What does it send to an AI?"** Nothing: `atfq ask` only prints. The
  environment is never dumped, only a short list of well-known variables,
  because environments hold tokens.
- **"Which distributions?"** Debian, Ubuntu, Fedora and Arch are the
  targets; the probes were checked against the files of Debian 13, Ubuntu
  24.04 and Fedora 44, and run live on Arch ARM. It compiles for x86-64 and
  32-bit ARM.
- **"How do I add a card?"** A Markdown file in `content/`, a line in
  `content/SUMMARY.md`, and `cargo test`: a test checks ids, references,
  probes and the TL;DR length. `ROADMAP.md` section 9 is the writing guide.
