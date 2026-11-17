//! Running a compiled script.
//!
//! On Unix, `run` replaces the `func` process with the script (`exec`), so
//! there is no wrapper process: signals, stdin, exit codes and `ps` all
//! behave exactly as if the script had been started directly.

use crate::report::Problem;
use std::path::Path;
use std::process::{Command, ExitStatus};

#[cfg(unix)]
pub fn run(program: &Path, args: &[String]) -> Result<u8, Problem> {
    use std::os::unix::process::CommandExt;
    // `exec` only returns if it failed.
    let error = Command::new(program).args(args).exec();
    Err(spawn_problem(program, &error))
}

#[cfg(not(unix))]
pub fn run(program: &Path, args: &[String]) -> Result<u8, Problem> {
    wait(program, args)
}

/// Runs a program to completion and reports its exit code. Used where
/// `func` must keep control afterwards (the test runner).
pub fn wait(program: &Path, args: &[String]) -> Result<u8, Problem> {
    let status = Command::new(program).args(args).status().map_err(|e| spawn_problem(program, &e))?;
    Ok(code(status))
}

/// Shell convention: a normal exit keeps its code, a signal becomes 128 + signal.
pub fn code(status: ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code & 0xff).unwrap_or(1);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return u8::try_from(128 + signal).unwrap_or(1);
        }
    }
    1
}

pub fn spawn_problem(program: &Path, error: &std::io::Error) -> Problem {
    Problem::new("CANNOT START PROGRAM")
        .text("I could not start:")
        .code(program.display().to_string())
        .text(format!("The operating system said: {error}"))
        .hint("If you just cleaned the cache or moved things around, run the command again to rebuild.")
}
