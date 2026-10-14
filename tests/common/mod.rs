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

/// The `laptop` fixture, with filesystem space for its mounts.
pub fn laptop() -> ProbeCtx {
    fixture("laptop")
        .with_space(
            "/",
            FsSpace {
                total: 400 * GIB,
                available: 120 * GIB,
            },
        )
        .with_space(
            "/boot/efi",
            FsSpace {
                total: GIB / 2,
                available: GIB / 4,
            },
        )
}

/// Renders a query the way `idk` prints it, against the laptop fixture.
pub fn show(lib: &Library, query: &str, opts: Options) -> String {
    let l = lookup(lib, query, &laptop());
    render::lookup(lib, &l, opts).unwrap_or_else(|| panic!("`{query}` matched nothing"))
}
