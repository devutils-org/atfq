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
        ("memory", "17. Memory  hub"),
        ("disk", "14. Disk  hub"),
        ("free", "17.1  free vs available"),
        ("zombie", "16.1  Process states and zombies"),
        ("find", "28.1  find"),
        ("gpu", "25. GPU  hub"),
        ("logs", "26. Logs  hub"),
        ("git", "30.1  How git stores your work"),
        ("docker", "7.1  What a container really is"),
        (
            "too many open files",
            "8.4  Too many open files: limits on processes",
        ),
        ("utc", "10.3  Time zones, UTC and daylight saving"),
        (
            "mojibake",
            "12.1  Bytes and characters: ASCII, Unicode and UTF-8",
        ),
        ("17.1", "17.1  free vs available"),
        ("17", "17. Memory  hub"),
        ("RAM", "17. Memory  hub"),
        ("Memory   Full", "17.1  free vs available"),
        ("proc.states", "16.1  Process states and zombies"),
        ("processes", "16. Processes & scheduling  hub"),
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
    // "permissions" is the hub's id and an alias of card 15.1: the hub wins.
    let out = show(&lib(), "permissions", PLAIN);
    assert!(
        out.starts_with("Looking for \"rwx, octal and umask\"? Run atfq 15.1\n"),
        "{out}"
    );
    assert!(out.contains("15. Permissions  hub"));
}

#[test]
fn deep_dive_only_with_why() {
    let lib = lib();
    let question = "Why is unused RAM wasted RAM?";
    let short = show(&lib, "free", PLAIN);
    assert!(!short.contains(question));
    assert!(short.contains("Deep dive: 3 questions. Read them with atfq 17.1 --why"));
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
   9. How filesystems work
      9.1  How a filesystem stores a file
      9.2  Journaling: how a filesystem survives a crash
      9.3  Copy-on-write filesystems: snapshots and checksums
      9.4  ext4, XFS, btrfs or ZFS?
      9.5  The VFS: one interface for every filesystem
  10. How time works
      10.1  Wall clock and monotonic clock: measuring time right
      10.2  Keeping the clock right: NTP
      10.3  Time zones, UTC and daylight saving
      10.4  Timestamps: Unix time, 2038 and leap seconds
  11. How cryptography works
      11.1  Hashes, encryption and signatures
      11.2  Public-key cryptography
      11.3  Certificates: how HTTPS proves who it is
      11.4  Random numbers: /dev/urandom and entropy
      11.5  Disk encryption: LUKS and the TPM
  12. How text is stored
      12.1  Bytes and characters: ASCII, Unicode and UTF-8
      12.2  Locales: why sorting, dates and numbers change
      12.3  Line endings, BOMs and invisible characters

Part II · Your system
  13. Filesystem
      13.1  The hierarchy in 2 minutes
      13.2  /tmp vs /var/tmp
      13.3  /proc and /sys
      13.4  Links, inodes and \"disk full\"
      13.5  fstab and mount options
      13.6  /dev and udev
  14. Disk  hub: atfq disk
  15. Permissions  hub: atfq permissions
      15.1  rwx, octal and umask
      15.2  setuid, setgid and sticky bit
      15.3  sudo and groups
      15.4  ACLs and capabilities
      15.5  SELinux and AppArmor
  16. Processes & scheduling  hub: atfq processes
      16.1  Process states and zombies
      16.2  Signals
      16.3  nice and priorities
      16.4  cgroups
  17. Memory  hub: atfq memory
      17.1  free vs available
      17.2  The OOM killer
      17.3  Swap and swappiness
      17.4  The page cache
  18. CPU  hub: atfq cpu
      18.1  Load average
  19. Network  hub: atfq network
      19.1  Who listens on which port
      19.2  DNS resolution
      19.3  Interfaces and routes
      19.4  nftables vs iptables vs ufw
  20. Boot  hub: atfq boot
      20.1  From firmware to init
      20.2  The kernel command line
      20.3  GRUB vs systemd-boot vs UKI
      20.4  systemd targets
  21. Kernel
      21.1  sysctl
      21.2  Kernel modules
      21.3  dmesg and the kernel log
      21.4  syscalls and strace
  22. systemd
      22.1  Units and timers
      22.2  Why a service won't start
      22.3  Unit files and overrides
  23. Libraries & binaries
      23.1  Dynamic linking and ldd
      23.2  ELF and \"command not found\"
      23.3  PATH and command resolution
  24. Packages
      24.1  Which package owns a file
      24.2  Native packages vs Snap and Flatpak
  25. GPU  hub: atfq gpu
  26. Logs  hub: atfq logs
      26.1  Reading the journal with journalctl
      26.2  Where logs go: journald, syslog and /var/log
      26.3  Log rotation and logs that fill the disk
      26.4  Logging from your own programs and services

Part III · Shell and commands
  27. Environment & shell
      27.1  Where environment variables come from
      27.2  Shell startup files
      27.3  Redirections and file descriptors
      27.4  Exit codes
  28. Finding and transforming
      28.1  find
      28.2  grep and regex
      28.3  awk
      28.4  sed
      28.5  xargs
      28.6  tar
      28.7  rsync
      28.8  jq
      28.9  sort, uniq and cut
  29. Remote work: SSH and tmux
      29.1  SSH keys and the agent
      29.2  ~/.ssh/config, jump hosts and host keys
      29.3  SSH tunnels and port forwarding
      29.4  tmux: sessions that survive a lost connection
      29.5  Copying files over SSH: scp, sftp and rsync
  30. Git
      30.1  How git stores your work
      30.2  Undoing things in git
      30.3  Merge or rebase?
      30.4  Remotes: fetch, pull and push
      30.5  Configuring git
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

fn atfq(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_atfq"))
        .args(args)
        .output()
        .expect("run atfq");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn cli_exit_codes() {
    let (code, out, _) = atfq(&["free"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("17.1  free vs available"));
    // Not a TTY: no colors.
    assert!(!out.contains('\x1b'));

    let (code, out, err) = atfq(&["nothing", "here"]);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(err.contains("Nothing matches \"nothing here\""));

    assert_eq!(atfq(&["--bogus"]).0, 2);
    assert_eq!(atfq(&[]).0, 2);
    assert_eq!(atfq(&["toc", "extra"]).0, 2);
}

#[test]
fn cli_why_and_toc() {
    let (code, out, _) = atfq(&["free", "--why"]);
    assert_eq!(code, 0);
    assert!(out.contains("Why is unused RAM wasted RAM?"));
    let (code, out, _) = atfq(&["toc"]);
    assert_eq!(code, 0);
    assert!(out.contains("17.1  free vs available"));
}
