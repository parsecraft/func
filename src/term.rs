//! Terminal capability checks, minimal ANSI styling, and the two bits of
//! interaction (`confirm`, `write_stdout`) the rest of the program needs.

use std::io::{BufRead, IsTerminal, Write};

#[derive(Clone, Copy)]
pub enum Style {
    Bold,
    Dim,
    Cyan,
}

impl Style {
    fn code(self) -> &'static str {
        match self {
            Style::Bold => "1",
            Style::Dim => "2",
            Style::Cyan => "36",
        }
    }
}

pub fn paint(text: &str, style: Style, enabled: bool) -> String {
    if enabled { format!("\x1b[{}m{text}\x1b[0m", style.code()) } else { text.to_string() }
}

/// Color is on only for interactive terminals, and `NO_COLOR` always wins.
fn color_enabled(is_terminal: bool) -> bool {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
    is_terminal && !no_color && !dumb
}

pub fn stderr_color() -> bool {
    color_enabled(std::io::stderr().is_terminal())
}

pub fn stdout_color() -> bool {
    color_enabled(std::io::stdout().is_terminal())
}

pub fn stdin_is_terminal() -> bool {
    std::io::stdin().is_terminal()
}

/// Writes to stdout, ignoring errors. A closed pipe (`func list | head`)
/// is a normal way for output to end, not something to report.
pub fn write_stdout(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// Asks a yes/no question on stderr. End of input counts as "no", so a
/// closed terminal can never trap the caller in a loop.
pub fn confirm(question: &str, default_yes: bool) -> bool {
    let choices = if default_yes { "[Y/n]" } else { "[y/N]" };
    eprint!("{question} {choices} ");
    let _ = std::io::stderr().flush();

    let mut line = String::new();
    match std::io::stdin().lock().read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => match line.trim().to_ascii_lowercase().as_str() {
            "" => default_yes,
            answer => answer == "y" || answer == "yes",
        },
    }
}
