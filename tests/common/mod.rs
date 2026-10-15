//! Shared helpers for the integration tests.
#![allow(dead_code)]

use std::path::Path;

use atfq::content::Library;
use atfq::probes::{FsSpace, ProbeCtx};
use atfq::render::{self, Options};
use atfq::view::lookup;

pub const PLAIN: Options = Options {
    why: false,
    color: false,
};
pub const WHY: Options = Options {
    why: true,
    color: false,
};

const GIB: u64 = 1 << 30;

pub fn lib() -> Library {
    Library::embedded().unwrap_or_else(|e| panic!("{e}"))
}

/// A fixture tree from `tests/fixtures/`.
pub fn fixture(name: &str) -> ProbeCtx {
    ProbeCtx::fixture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
}

fn space(total: u64, available: u64, inodes_total: u64, inodes_free: u64) -> FsSpace {
    FsSpace {
        total,
        available,
        inodes_total,
        inodes_free,
    }
}

/// The `laptop` fixture, with filesystem space and IP addresses.
pub fn laptop() -> ProbeCtx {
    fixture("laptop")
        .with_space("/", space(400 * GIB, 120 * GIB, 0, 0))
        .with_space("/boot/efi", space(GIB / 2, GIB / 4, 0, 0))
        .with_space("/tmp", space(8 * GIB, 7 * GIB, 2_000_000, 1_998_000))
        .with_addr("lo", "127.0.0.1/8")
        .with_addr("lo", "::1/128")
        .with_addr("wlp0s20f3", "192.168.1.23/24")
        .with_addr("wlp0s20f3", "fe80::8ec6:81ff:fe2a:107f/64")
        .with_addr("docker0", "172.17.0.1/16")
        .with_mode("/home/alice/.ssh", 0o700)
        .with_mode("/home/alice/.ssh/id_ed25519", 0o600)
        .with_mode("/home/alice/.ssh/id_rsa_old", 0o644)
        .with_command(&["systemctl", "is-system-running"], "degraded\n")
        .with_command(
            &[
                "systemctl",
                "list-units",
                "--state=failed",
                "--no-legend",
                "--plain",
                "--no-pager",
            ],
            "backup.service loaded failed failed Nightly backup\n",
        )
        .with_command(
            &[
                "systemctl",
                "is-active",
                "firewalld.service",
                "ufw.service",
                "nftables.service",
            ],
            "inactive\nactive\ninactive\n",
        )
}

/// Renders a query the way `idk` prints it, against the laptop fixture.
pub fn show(lib: &Library, query: &str, opts: Options) -> String {
    let l = lookup(lib, query, &laptop());
    render::lookup(lib, &l, opts).unwrap_or_else(|| panic!("`{query}` matched nothing"))
}
