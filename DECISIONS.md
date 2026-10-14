# Decisions

Decisions taken during development that ROADMAP.md does not cover, one line each: date, decision, reason.


- 2026-10-14 — SUMMARY.md order is Filesystem, Disk, Permissions, Processes & scheduling, Memory, Finding and transforming. Filesystem and Permissions have no cards yet. — The phase 0 acceptance (`idk 5.1`) and the TUI mockup put Memory at chapter 5; the section 6 example would make it 3.
- 2026-10-14 — In SUMMARY.md, a hub can only be a chapter page and a card can only be an entry under a chapter; the titles in SUMMARY.md must match the frontmatter titles. — Stops the TOC and the cards from drifting apart.
- 2026-10-14 — `probes::REGISTRY` lists probe names (with no functions) from phase 0. — The 6.3 validation must check probe names before the probes exist; the functions come in phase 1.
- 2026-10-14 — Kept `serde_yaml` 0.9 although it is deprecated upstream. — The roadmap names it and it works. Switch to a maintained fork if it causes trouble.
- 2026-10-14 — Content uses fenced code blocks (```), never indented ones. — In CommonMark, an indented block right after a list becomes a paragraph of the last list item.
- 2026-10-14 — TL;DR length = non-blank source lines, code fences excluded. — Close to what the screen shows, and cheap to check.
- 2026-10-14 — Validation beyond 6.3: unknown frontmatter fields are errors, and an alias can't be a subcommand name (`toc`, `search`…) or look like a chapter number. — Catches typos and keys that could never be reached.
- 2026-10-14 — `idk 5` (a bare chapter number) opens that chapter's hub when it has one, and otherwise matches nothing. — Natural extension of resolution step 3.
- 2026-10-14 — The no-match message suggests `idk toc` until `search` and `ask` exist. — Never point users to a command that isn't there yet.
- 2026-10-14 — Exit code 3 = internal error (invalid embedded content, failed write to stdout); a closed pipe is not an error. — 0/1/2 are taken by success, no match and usage error.
- 2026-10-14 — `build.rs` with `rerun-if-changed=content`. — `include_dir!` does not see new files without it.
- 2026-10-14 — A hub page lists its own chapter's cards first, then its `cards` field. — Otherwise the Memory hub would not link to 5.1.
- 2026-10-14 — Text width = number of chars, without `unicode-width`. — Content is mostly ASCII; avoids a dependency.
