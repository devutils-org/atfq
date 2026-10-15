use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use serde::Serialize;

use atfq::content::Library;
use atfq::probes::{self, ProbeCtx};
use atfq::prompt::{self, SystemInfo};
use atfq::render::{self, Options};
use atfq::search::search;
use atfq::view::{self, lookup, Match};
use atfq::BIN_NAME;

/// Short answers about Linux, from your terminal.
#[derive(Parser)]
#[command(
    name = BIN_NAME,
    version,
    args_conflicts_with_subcommands = true
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
    /// Print a prompt to paste into an AI assistant, with your machine's data
    Ask {
        /// What the question is about: a hub, a topic or a chapter number
        query: String,
        /// Your question, in your own words
        #[arg(value_name = "QUESTION")]
        text: Vec<String>,
        /// Leave out the hostname, the username and home directory paths
        #[arg(long)]
        redact: bool,
    },
    /// Serve the knowledge and probes to AI agents over MCP (stdio, read-only)
    Mcp,
    /// Browse interactively (also what `atfq` alone does in a terminal)
    Tui {
        /// A page to open first
        query: Vec<String>,
    },
}

/// Exit codes: 0 success, 1 no match, 2 usage error (clap), 3 internal error.
const NO_MATCH: u8 = 1;
const USAGE: u8 = 2;
const INTERNAL: u8 = 3;

/// Results shown by `atfq search`.
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

    let tty = io::stdin().is_terminal() && io::stdout().is_terminal();
    let (out, code) = match cli.command {
        Some(Command::Tui { query }) => return tui(&lib, ctx, &query),
        Some(Command::Mcp) => {
            return match atfq::mcp::serve(lib, ctx) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{BIN_NAME}: mcp: {e}");
                    ExitCode::from(INTERNAL)
                }
            }
        }
        None if cli.query.is_empty() && tty => return tui(&lib, ctx, &[]),
        None if cli.query.is_empty() => {
            // Not a terminal: print the help, as for a usage error.
            let _ = Cli::command().write_help(&mut io::stderr());
            return ExitCode::from(USAGE);
        }
        Some(Command::Ask {
            query,
            text,
            redact,
        }) => {
            let l = lookup(&lib, &query, &ctx);
            let sys = SystemInfo::collect(&ctx);
            let text = text.join(" ");
            let p = prompt::build(&l, Some(&text), &sys, redact);
            eprintln!("{}", prompt::summary(&p, &l));
            (either(cli.json, &p, || p.text.clone()), ExitCode::SUCCESS)
        }
        Some(Command::Toc) => (
            either(cli.json, &view::toc(&lib), || render::toc(&lib, opts)),
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
    // A closed pipe (`atfq find | head`) is not an error worth reporting.
    match written {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            eprintln!("{BIN_NAME}: {e}");
            ExitCode::from(INTERNAL)
        }
        _ => code,
    }
}

fn tui(lib: &Library, ctx: ProbeCtx, query: &[String]) -> ExitCode {
    let sys = SystemInfo::collect(&ctx);
    let start = (!query.is_empty()).then(|| query.join(" "));
    match atfq::tui::run(lib, ctx, sys, start.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{BIN_NAME}: terminal error: {e}");
            ExitCode::from(INTERNAL)
        }
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
