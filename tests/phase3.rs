//! Phase 3 acceptance: the AI prompt built by `idk ask`.

mod common;

use std::process::Command;

use atfq::prompt::{build, SystemInfo};
use atfq::view::lookup;
use common::{laptop, lib};
use insta::assert_snapshot;

fn sys() -> SystemInfo {
    SystemInfo {
        distro: Some("Debian GNU/Linux 13 (trixie)".into()),
        kernel: Some("6.12.48-amd64".into()),
        arch: "x86_64".into(),
        hostname: Some("alice-laptop".into()),
        user: Some("alice".into()),
        home: Some("/home/alice".into()),
    }
}

#[test]
fn prompt_for_a_card() {
    let lib = lib();
    let l = lookup(&lib, "memory", &laptop());
    let p = build(&l, Some("why is my laptop swapping?"), &sys(), false);
    assert_snapshot!("ask_memory", p.text);
    assert_eq!(
        p.included,
        [
            "distribution",
            "kernel version",
            "architecture",
            "live data (mem.summary)"
        ]
    );
}

#[test]
fn default_question_and_no_match() {
    let lib = lib();
    let l = lookup(&lib, "12.1", &laptop());
    let p = build(&l, None, &sys(), false);
    assert!(p
        .text
        .contains("Help me understand \"Process states and zombies\""));
    assert!(p.text.contains("PID 4242 worker (old)"));

    let l = lookup(&lib, "qwertyuiop", &laptop());
    let p = build(&l, Some("how do I list USB devices?"), &sys(), false);
    assert!(!p.text.contains("## Background"));
    assert!(!p.text.contains("## What my machine shows"));
    assert!(p.text.contains("how do I list USB devices?"));
}

#[test]
fn redact_hides_host_user_and_home() {
    let lib = lib();
    let l = lookup(&lib, "disk", &laptop());
    let question = "alice here: why is /home/alice/videos full on alice-laptop? (and /home/bob)";
    let plain = build(&l, Some(question), &sys(), false);
    assert!(plain.text.contains("alice"));

    let p = build(&l, Some(question), &sys(), true);
    for secret in ["alice", "alice-laptop", "/home/alice", "/home/bob"] {
        assert!(!p.text.contains(secret), "{secret} leaked:\n{}", p.text);
    }
    assert!(p
        .text
        .contains("<user> here: why is ~/videos full on <hostname>?"));
    assert!(p.redacted && !plain.redacted);
}

#[test]
fn cli_ask() {
    let out = Command::new(env!("CARGO_BIN_EXE_idk"))
        .args(["ask", "memory", "why", "is", "my", "laptop", "swapping?"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("why is my laptop swapping?"));
    assert!(stdout.contains("## What my machine shows right now"));
    // One line on stderr saying what is included; the prompt alone on stdout.
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("Included: "), "{stderr}");
    assert!(!stdout.contains("Included: "));
}

#[test]
fn cli_ask_redact() {
    let user = std::env::var("USER").unwrap_or_default();
    let out = Command::new(env!("CARGO_BIN_EXE_idk"))
        .args([
            "ask",
            "disk",
            "--redact",
            &format!("what is in /home/{user}?"),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    if user.len() > 2 {
        assert!(!stdout.contains(&user), "{stdout}");
    }
    assert!(!stdout.contains("/home/") || stdout.contains("/home/<user>"));
    if let Ok(host) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
        let host = host.trim();
        if host.len() > 2 {
            assert!(!stdout.contains(host), "{stdout}");
        }
    }
}
