//! Phase 0 acceptance: the embedded content is valid, every query from the
//! roadmap lands on the right page, and the CLI behaves.

mod common;

use std::process::Command;

use atfq::probes::run_all;
use atfq::render;
use atfq::resolve::{resolve, Resolved};
use common::{laptop, lib, show, PLAIN, WHY};

#[test]
fn embedded_content_is_valid() {
    // Library::embedded runs every rule of ROADMAP.md section 6.3.
    lib();
}

#[test]
fn queries_reach_the_right_page() {
    let lib = lib();
    for (query, first_line) in [
        ("memory", "5. Memory  hub"),
        ("disk", "2. Disk  hub"),
        ("free", "5.1  free vs available"),
        ("zombie", "4.1  Process states and zombies"),
        ("find", "6.1  find"),
        ("5.1", "5.1  free vs available"),
        ("5", "5. Memory  hub"),
        ("RAM", "5. Memory  hub"),
        ("Memory   Full", "5.1  free vs available"),
        ("proc.states", "4.1  Process states and zombies"),
    ] {
        let out = show(&lib, query, PLAIN);
        assert_eq!(out.lines().next(), Some(first_line), "query `{query}`");
    }
}

#[test]
fn unknown_queries_match_nothing() {
    let lib = lib();
    for query in ["nope", "9.9", "1", "5.0", "0.1", ""] {
        assert!(
            matches!(resolve(&lib, query), Resolved::NotFound),
            "`{query}` should not match"
        );
    }
}

#[test]
fn deep_dive_only_with_why() {
    let lib = lib();
    let question = "Why is unused RAM wasted RAM?";
    let short = show(&lib, "free", PLAIN);
    assert!(!short.contains(question));
    assert!(short.contains("Deep dive: 3 questions. Read them with idk 5.1 --why"));
    let long = show(&lib, "free", WHY);
    assert!(long.contains(question));
    assert!(long.contains("What is swappiness and when should I change it?"));
}

#[test]
fn live_block_shows_probe_results() {
    let lib = lib();
    let out = show(&lib, "free", PLAIN);
    assert!(
        out.contains("On your machine\n  RAM       10.5 GiB available of 15.5 GiB"),
        "{out}"
    );
    // `find` has no probe: no live block at all.
    assert!(!show(&lib, "find", PLAIN).contains("On your machine"));
}

#[test]
fn toc_follows_summary() {
    let toc = render::toc(&lib(), PLAIN);
    let expected = "\
Chapters
  1. Filesystem
     1.1  The hierarchy in 2 minutes
     1.2  /tmp vs /var/tmp
     1.3  /proc and /sys
     1.4  Links, inodes and \"disk full\"
  2. Disk  hub: idk disk
  3. Permissions
     3.1  rwx, octal and umask
     3.2  setuid, setgid and sticky bit
     3.3  sudo and groups
  4. Processes & scheduling
     4.1  Process states and zombies
     4.2  Signals
     4.3  nice and priorities
  5. Memory  hub: idk memory
     5.1  free vs available
     5.2  The OOM killer
     5.3  Swap and swappiness
  6. Finding and transforming
     6.1  find
  7. CPU  hub: idk cpu
     7.1  Load average
  8. Network  hub: idk network
     8.1  Who listens on which port
     8.2  DNS resolution
  9. Boot  hub: idk boot
     9.1  From firmware to init
     9.2  The kernel command line
";
    assert!(toc.ends_with(expected), "{toc}");
}

#[test]
fn output_fits_80_columns_and_has_no_prompt_signs() {
    let lib = lib();
    let ctx = laptop();
    let mut pages = vec![render::toc(&lib, WHY)];
    pages.extend(
        lib.hubs()
            .map(|h| render::hub(&lib, h, None, &run_all(&h.probes, &ctx), WHY)),
    );
    pages.extend(
        lib.cards()
            .map(|c| render::card(&lib, c, &run_all(&c.probes, &ctx), WHY)),
    );
    for page in pages {
        for line in page.lines() {
            assert!(line.chars().count() <= 80, "too wide: {line:?}");
            assert!(
                !line.trim_start().starts_with("$ "),
                "prompt sign: {line:?}"
            );
        }
    }
}

#[test]
fn dangerous_commands_are_marked() {
    let lib = lib();
    let find = lib.card("cmd.find").unwrap();
    // Not in the card itself, so check through a rendered snippet.
    let mut card = find.clone();
    card.tldr = "```\nfind . -name '*.tmp' -delete\n```".into();
    let out = render::card(&lib, &card, &[], PLAIN);
    assert!(out.contains("⚠ changes or deletes data"), "{out}");
}

fn idk(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_idk"))
        .args(args)
        .output()
        .expect("run idk");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn cli_exit_codes() {
    let (code, out, _) = idk(&["free"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("5.1  free vs available"));
    // Not a TTY: no colors.
    assert!(!out.contains('\x1b'));

    let (code, out, err) = idk(&["nothing", "here"]);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(err.contains("Nothing matches \"nothing here\""));

    assert_eq!(idk(&["--bogus"]).0, 2);
    assert_eq!(idk(&[]).0, 2);
    assert_eq!(idk(&["toc", "extra"]).0, 2);
}

#[test]
fn cli_why_and_toc() {
    let (code, out, _) = idk(&["free", "--why"]);
    assert_eq!(code, 0);
    assert!(out.contains("Why is unused RAM wasted RAM?"));
    let (code, out, _) = idk(&["toc"]);
    assert_eq!(code, 0);
    assert!(out.contains("5.1  free vs available"));
}
