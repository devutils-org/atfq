use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use atfq::content::Library;
use atfq::render::{self, Options};
use atfq::resolve::{resolve, Resolved};
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the table of contents
    Toc,
}

/// Exit codes: 0 success, 1 no match, 2 usage error (clap), 3 internal error.
const NO_MATCH: u8 = 1;
const INTERNAL: u8 = 3;

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

    let (text, code) = match cli.command {
        Some(Command::Toc) => (render::toc(&lib, opts), ExitCode::SUCCESS),
        None => {
            let query = cli.query.join(" ");
            match resolve(&lib, &query) {
                Resolved::Hub { hub, also_card } => {
                    (render::hub(&lib, hub, also_card, opts), ExitCode::SUCCESS)
                }
                Resolved::Card(card) => (render::card(&lib, card, opts), ExitCode::SUCCESS),
                Resolved::NotFound => {
                    eprint!("{}", render::not_found(&query, opts));
                    return ExitCode::from(NO_MATCH);
                }
            }
        }
    };
    // A closed pipe (`idk find | head`) is not an error worth reporting.
    match io::stdout().lock().write_all(text.as_bytes()) {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            eprintln!("{BIN_NAME}: {e}");
            ExitCode::from(INTERNAL)
        }
        _ => code,
    }
}

/// Colors only on a terminal, and never when NO_COLOR is set (no-color.org).
fn color_enabled() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}
