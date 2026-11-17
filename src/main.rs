//! func: write, test and run Rust scripts on the fly.
//!
//! Data flows one way through small modules:
//!
//! ```text
//! argv -> cli::parse -> commands::dispatch -> (store, script, build, exec)
//!                                   \-> Problem -> report -> stderr + exit code
//! ```
//!
//! Every failure the person can see is a [`report::Problem`]; nothing else
//! prints errors.

mod build;
mod cli;
mod commands;
mod completions;
mod editor;
mod exec;
mod hash;
mod paths;
mod report;
mod script;
mod store;
mod suggest;
mod term;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Lossy conversion: a non-UTF-8 argument becomes a "not found" message, never a panic.
    let args = std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();

    match cli::parse(args).and_then(commands::dispatch) {
        Ok(code) => ExitCode::from(code),
        Err(problem) => {
            problem.print();
            ExitCode::from(problem.exit_code())
        }
    }
}
