# Contributing to atfq

Thanks for helping. atfq teaches people how their system works, so a
wrong sentence costs more here than a bug in most tools: corrections to
cards are the most valuable contributions.

## Reporting a mistake

Open an issue with the "Mistake in a card" template: the card number, what
it says, what's right, and a source. If it depends on the distribution or
the kernel version, say which.

## Setting up

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All three must pass; CI runs them on x86-64 and ARM64. The toolchain is
pinned in `rust-toolchain.toml`.

Snapshot tests (`tests/snapshots/`) hold the rendered pages. After a
change to content or rendering, run `INSTA_UPDATE=always cargo test` and
read the diff of every snapshot before committing.

## Adding or fixing a card

Cards are Markdown files in `content/cards/`, hubs in `content/hubs/`.
[`docs/ROADMAP.md`](docs/ROADMAP.md) section 6 describes the format and section 9 the writing
guide; the essentials:

- Start from a question a developer actually has, or an error message they
  saw. Answer first (TL;DR), then pitfalls, then the deep dive.
- TL;DR at most 12 lines, Pitfalls at most 6 bullets, each deep dive
  question about 150 words. Pages must fit in 80 columns.
- Every command is real and runnable. Mark nothing as dangerous by hand:
  the renderer flags destructive commands itself (`src/content/danger.rs`);
  add a rule there if it misses one.
- Cite other cards by id, `{{memory.oom-killer}}`, never by number: numbers
  are filled in at load time.
- Verify every fact. If a behaviour depends on the version, say since
  when. If you're not sure, leave it out.
- No Markdown tables: the terminal renderer doesn't draw them.

A new card also needs a line in `content/SUMMARY.md`. **Add new chapters
and cards at the end of their part or chapter**: chapter numbers are
addresses people type and quote, and inserting renumbers everything after.

`cargo test` validates the whole library: unique ids and aliases, known
references and probes, section lengths, SUMMARY coverage.

## Adding a probe

Probes live in `src/probes/`. The rules:

- Read only, and only through `ProbeCtx` (`ctx.read`, `ctx.list`…), so
  tests can point it at a fixture tree in `tests/fixtures/`.
- Never panic on a missing, unreadable or malformed file: return a status
  (`Partial`, `NeedsRoot`, `Unavailable`) and a note saying why.
- No new external programs. `systemctl` is the only one, with fixed
  arguments and a timeout; anything else needs a discussion first.
- Never read or show secrets: no private keys, credentials, tokens, or the
  full environment. Show whether something is set, not its value, when the
  value is personal.
- Add fixture files and a unit test, and register the probe in
  `REGISTRY` with a one-line description.

## Commits and pull requests

One topic per pull request, with tests. Decisions the roadmap doesn't
cover go in [`docs/DECISIONS.md`](docs/DECISIONS.md), one line each: date — decision — reason.

By contributing, you agree that your contribution is dual licensed under
the MIT and Apache-2.0 licenses, like the rest of the project.
