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
        ("memory", "13. Memory  hub"),
        ("disk", "10. Disk  hub"),
        ("free", "13.1  free vs available"),
        ("zombie", "12.1  Process states and zombies"),
        ("find", "23.1  find"),
        ("gpu", "21. GPU  hub"),
        ("git", "25.1  How git stores your work"),
        ("docker", "7.1  What a container really is"),
        (
            "too many open files",
            "8.4  Too many open files: limits on processes",
        ),
        ("13.1", "13.1  free vs available"),
        ("13", "13. Memory  hub"),
        ("RAM", "13. Memory  hub"),
        ("Memory   Full", "13.1  free vs available"),
        ("proc.states", "12.1  Process states and zombies"),
        ("processes", "12. Processes & scheduling  hub"),
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
    // "permissions" is the hub's id and an alias of card 11.1: the hub wins.
    let out = show(&lib(), "permissions", PLAIN);
    assert!(
        out.starts_with("Looking for \"rwx, octal and umask\"? Run idk 11.1\n"),
        "{out}"
    );
    assert!(out.contains("11. Permissions  hub"));
}

#[test]
fn deep_dive_only_with_why() {
    let lib = lib();
    let question = "Why is unused RAM wasted RAM?";
    let short = show(&lib, "free", PLAIN);
    assert!(!short.contains(question));
    assert!(short.contains("Deep dive: 3 questions. Read them with idk 13.1 --why"));
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
      1.1  What does the kernel do?
      1.2  How the kernel shares the CPUs
      1.3  How virtual memory works
      1.4  Interrupts: how devices get attention
      1.5  How a program starts: fork and exec
   2. How a CPU works
      2.1  What a CPU does: instructions and architectures
      2.2  Cores, threads and sockets
      2.3  CPU caches: why memory layout matters
      2.4  Clock speed, turbo and power saving
      2.5  Speculative execution and CPU flaws
   3. How memory works
      3.1  RAM and the memory hierarchy
      3.2  NUMA: when some memory is closer
      3.3  ECC and failing RAM
   4. How storage works
      4.1  Hard drives, SSDs and NVMe
      4.2  Inside an SSD: flash, TRIM and wear
      4.3  From write() to the disk: the block layer
      4.4  IOPS, throughput and latency: how fast is a disk?
      4.5  fsync: when is data really on disk?
   5. How a GPU works
      5.1  What a GPU does differently
      5.2  GPU drivers: the kernel, Mesa and NVIDIA
      5.3  How a picture reaches the screen: KMS, Wayland and X11
      5.4  Computing on the GPU, and when your program can't see it
   6. How networks work
      6.1  Network layers: OSI and TCP/IP
      6.2  What happens when you open a website
      6.3  IP addresses, subnets and NAT
      6.4  TCP, UDP and connections
      6.5  Ethernet, MAC addresses and ARP
   7. How containers and virtual machines work
      7.1  What a container really is
      7.2  Namespaces: a private view of the system
      7.3  Images, layers and overlayfs
      7.4  Virtual machines and hypervisors
      7.5  Root in a container: users, capabilities and the docker group
   8. How programs do several things at once
      8.1  Processes, threads and async: doing several things at once
      8.2  Race conditions, locks and atomics
      8.3  Event loops: one thread, thousands of connections
      8.4  Too many open files: limits on processes
      8.5  How processes talk: sockets, shared memory and D-Bus

Part II · Your system
   9. Filesystem
      9.1  The hierarchy in 2 minutes
      9.2  /tmp vs /var/tmp
      9.3  /proc and /sys
      9.4  Links, inodes and \"disk full\"
      9.5  fstab and mount options
      9.6  /dev and udev
  10. Disk  hub: idk disk
  11. Permissions  hub: idk permissions
      11.1  rwx, octal and umask
      11.2  setuid, setgid and sticky bit
      11.3  sudo and groups
      11.4  ACLs and capabilities
      11.5  SELinux and AppArmor
  12. Processes & scheduling  hub: idk processes
      12.1  Process states and zombies
      12.2  Signals
      12.3  nice and priorities
      12.4  cgroups
  13. Memory  hub: idk memory
      13.1  free vs available
      13.2  The OOM killer
      13.3  Swap and swappiness
      13.4  The page cache
  14. CPU  hub: idk cpu
      14.1  Load average
  15. Network  hub: idk network
      15.1  Who listens on which port
      15.2  DNS resolution
      15.3  Interfaces and routes
      15.4  nftables vs iptables vs ufw
  16. Boot  hub: idk boot
      16.1  From firmware to init
      16.2  The kernel command line
      16.3  GRUB vs systemd-boot vs UKI
      16.4  systemd targets
  17. Kernel
      17.1  sysctl
      17.2  Kernel modules
      17.3  dmesg and the kernel log
      17.4  syscalls and strace
  18. systemd
      18.1  Units and timers
      18.2  Why a service won't start
      18.3  Unit files and overrides
  19. Libraries & binaries
      19.1  Dynamic linking and ldd
      19.2  ELF and \"command not found\"
      19.3  PATH and command resolution
  20. Packages
      20.1  Which package owns a file
      20.2  Native packages vs Snap and Flatpak
  21. GPU  hub: idk gpu

Part III · Shell and commands
  22. Environment & shell
      22.1  Where environment variables come from
      22.2  Shell startup files
      22.3  Redirections and file descriptors
      22.4  Exit codes
  23. Finding and transforming
      23.1  find
      23.2  grep and regex
      23.3  awk
      23.4  sed
      23.5  xargs
      23.6  tar
      23.7  rsync
      23.8  jq
      23.9  sort, uniq and cut
  24. Remote work: SSH and tmux
      24.1  SSH keys and the agent
      24.2  ~/.ssh/config, jump hosts and host keys
      24.3  SSH tunnels and port forwarding
      24.4  tmux: sessions that survive a lost connection
      24.5  Copying files over SSH: scp, sftp and rsync
  25. Git
      25.1  How git stores your work
      25.2  Undoing things in git
      25.3  Merge or rebase?
      25.4  Remotes: fetch, pull and push
      25.5  Configuring git
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
    assert!(out.starts_with("13.1  free vs available"));
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
    assert!(out.contains("13.1  free vs available"));
}
