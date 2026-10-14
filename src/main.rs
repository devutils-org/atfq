use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;

use atfq::content::Library;
use atfq::probes::{self, ProbeCtx};
use atfq::render::{self, Options};
use atfq::search::search;
use atfq::view::{lookup, Match};
use atfq::BIN_NAME;

/// Short answers about Linux, from your terminal.
#[derive(Parser)]
#[command(
    name = BIN_NAME,
    version,
    args_conflicts_with_subcommands = true,
    arg_required_else_help = true
)]
struct Cli {
    /// A hub, a topic, a command or a chapter number: `memory`, `zombie`, `5.1`
    #[arg(value_name = "QUERY")]
    query: Vec<String>,

    /// Also show the deep dive
    #[arg(long)]
    why: bool,

    /// Print structured JSON instead of text
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the table of contents
    Toc,
    /// Search the text of every hub and card
    Search {
        #[arg(required = true, value_name = "WORDS")]
        words: Vec<String>,
    },
    /// Run one probe and show what it found; without a name, list the probes
    Probe { name: Option<String> },
}

/// Exit codes: 0 success, 1 no match, 2 usage error (clap), 3 internal error.
const NO_MATCH: u8 = 1;
const INTERNAL: u8 = 3;

/// Results shown by `idk search`.
const SEARCH_LIMIT: usize = 10;

/// What to print, and where.
enum Out {
    Stdout(String),
    Stderr(String),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let lib = match Library::embedded() {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("{BIN_NAME}: internal error: {e}");
            return ExitCode::from(INTERNAL);
        }
    };
    let opts = Options {
        why: cli.why,
        color: color_enabled(),
    };
    let ctx = ProbeCtx::system();

    let (out, code) = match cli.command {
        Some(Command::Toc) => (
            either(cli.json, lib.chapters(), || render::toc(&lib, opts)),
            ExitCode::SUCCESS,
        ),
        Some(Command::Search { words }) => {
            let query = words.join(" ");
            let hits = search(&lib, &query, SEARCH_LIMIT);
            let code = exit(!hits.is_empty());
            (
                either(cli.json, &hits, || render::search(&query, &hits, opts)),
                code,
            )
        }
        Some(Command::Probe { name: None }) => (
            either(cli.json, probes::REGISTRY, || {
                render::probe_list(probes::REGISTRY, opts)
            }),
            ExitCode::SUCCESS,
        ),
        Some(Command::Probe { name: Some(name) }) => match probes::find(&name) {
            Some(info) => {
                let r = info.run(&ctx);
                (
                    either(cli.json, &r, || render::probe(info, &r, opts)),
                    ExitCode::SUCCESS,
                )
            }
            None => (
                Out::Stderr(format!(
                    "No probe named \"{name}\".\n{}",
                    render::probe_list(probes::REGISTRY, opts)
                )),
                exit(false),
            ),
        },
        None => {
            let query = cli.query.join(" ");
            let l = lookup(&lib, &query, &ctx);
            let found = l.matched != Match::None;
            let out = if cli.json {
                Out::Stdout(json(&l))
            } else {
                match render::lookup(&lib, &l, opts) {
                    Some(page) => Out::Stdout(page),
                    None => Out::Stderr(render::not_found(&l, opts)),
                }
            };
            (out, exit(found))
        }
    };

    let written = match out {
        Out::Stdout(s) => io::stdout().lock().write_all(s.as_bytes()),
        Out::Stderr(s) => io::stderr().lock().write_all(s.as_bytes()),
    };
    // A closed pipe (`idk find | head`) is not an error worth reporting.
    match written {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            eprintln!("{BIN_NAME}: {e}");
            ExitCode::from(INTERNAL)
        }
        _ => code,
    }
}

fn exit(found: bool) -> ExitCode {
    if found {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(NO_MATCH)
    }
}

fn either<T: Serialize + ?Sized>(as_json: bool, value: &T, text: impl FnOnce() -> String) -> Out {
    Out::Stdout(if as_json { json(value) } else { text() })
}

fn json<T: Serialize + ?Sized>(value: &T) -> String {
    // Serializing plain structs and strings can't fail.
    let mut s = serde_json::to_string_pretty(value).expect("serializable");
    s.push('\n');
    s
}

/// Colors only on a terminal, and never when NO_COLOR is set (no-color.org).
fn color_enabled() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}
