//! atfq: the core of the `atfq` command. Surfaces (CLI, TUI, and later
//! MCP) only call into this library.

pub mod content;
pub mod mcp;
pub mod probes;
pub mod prompt;
pub mod render;
pub mod resolve;
pub mod search;
pub mod tui;
pub mod view;

/// The command users type. Help texts use this, never a literal.
pub const BIN_NAME: &str = "atfq";
