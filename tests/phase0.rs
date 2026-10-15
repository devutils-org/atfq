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
        ("memory", "11. Memory  hub"),
        ("disk", "8. Disk  hub"),
        ("free", "11.1  free vs available"),
        ("zombie", "10.1  Process states and zombies"),
        ("find", "20.1  find"),
        ("11.1", "11.1  free vs available"),
        ("11", "11. Memory  hub"),
        ("RAM", "11. Memory  hub"),
        ("Memory   Full", "11.1  free vs available"),
        ("proc.states", "10.1  Process states and zombies"),
        ("processes", "10. Processes & scheduling  hub"),
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
fn hub_points_to_card_sharing_its_name() {
    // "permissions" is the hub's id and an alias of card 3.1: the hub wins.
    let out = show(&lib(), "permissions", PLAIN);
    assert!(
        out.starts_with("Looking for \"rwx, octal and umask\"? Run idk 9.1\n"),
        "{out}"
    );
    assert!(out.contains("9. Permissions  hub"));
}

#[test]
fn deep_dive_only_with_why() {
    let lib = lib();
    let question = "Why is unused RAM wasted RAM?";
    let short = show(&lib, "free", PLAIN);
    assert!(!short.contains(question));
    assert!(short.contains("Deep dive: 3 questions. Read them with idk 11.1 --why"));
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
Part I · How it works
   1. How the kernel works
   2. How a CPU works
   3. How memory works
   4. How storage works
   5. How a GPU works
   6. How networks work

Part II · Your system
   7. Filesystem
      7.1  The hierarchy in 2 minutes
      7.2  /tmp vs /var/tmp
      7.3  /proc and /sys
      7.4  Links, inodes and \"disk full\"
      7.5  fstab and mount options
      7.6  /dev and udev
   8. Disk  hub: idk disk
   9. Permissions  hub: idk permissions
      9.1  rwx, octal and umask
      9.2  setuid, setgid and sticky bit
      9.3  sudo and groups
      9.4  ACLs and capabilities
      9.5  SELinux and AppArmor
  10. Processes & scheduling  hub: idk processes
      10.1  Process states and zombies
      10.2  Signals
      10.3  nice and priorities
      10.4  cgroups
  11. Memory  hub: idk memory
      11.1  free vs available
      11.2  The OOM killer
      11.3  Swap and swappiness
      11.4  The page cache
  12. CPU  hub: idk cpu
      12.1  Load average
  13. Network  hub: idk network
      13.1  Who listens on which port
      13.2  DNS resolution
      13.3  Interfaces and routes
      13.4  nftables vs iptables vs ufw
  14. Boot  hub: idk boot
      14.1  From firmware to init
      14.2  The kernel command line
      14.3  GRUB vs systemd-boot vs UKI
      14.4  systemd targets
  15. Kernel
      15.1  sysctl
      15.2  Kernel modules
      15.3  dmesg and the kernel log
      15.4  syscalls and strace
  16. systemd
      16.1  Units and timers
      16.2  Why a service won't start
      16.3  Unit files and overrides
  17. Libraries & binaries
      17.1  Dynamic linking and ldd
      17.2  ELF and \"command not found\"
      17.3  PATH and command resolution
  18. Packages
      18.1  Which package owns a file
      18.2  Native packages vs Snap and Flatpak

Part III · Shell and commands
  19. Environment & shell
      19.1  Where environment variables come from
      19.2  Shell startup files
      19.3  Redirections and file descriptors
      19.4  Exit codes
  20. Finding and transforming
      20.1  find
      20.2  grep and regex
      20.3  awk
      20.4  sed
      20.5  xargs
      20.6  tar
      20.7  rsync
      20.8  jq
      20.9  sort, uniq and cut
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
    assert!(out.starts_with("11.1  free vs available"));
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
    assert!(out.contains("11.1  free vs available"));
}
