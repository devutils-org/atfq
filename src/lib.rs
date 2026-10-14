//! atfq: the core behind the `idk` command. Surfaces (CLI, and later TUI and
//! MCP) only call into this library.

pub mod content;
pub mod probes;
pub mod render;
pub mod resolve;
pub mod search;
pub mod view;

/// The command users type. Help texts use this, never a literal.
pub const BIN_NAME: &str = "idk";
