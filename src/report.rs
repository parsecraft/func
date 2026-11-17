//! Every error the person can see goes through [`Problem`].
//!
//! The format is borrowed from Elm's compiler: a titled bar, a plain-language
//! explanation, a concrete next step. No stack traces, no jargon, no
//! "error: error".
//!
//! ```text
//! -- UNKNOWN SCRIPT ------------------------------------------ func run helo
//!
//! I could not find a script named `helo`.
//!
//! Did you mean one of these?
//!
//!     hello
//!
//! Hint: Create it with `func edit helo`.
//! ```

use crate::term::{self, Style};
use std::path::Path;

const WIDTH: usize = 80;

#[derive(Debug)]
enum Block {
    /// A paragraph.
    Text(String),
    /// Something to copy: indented by four spaces.
    Code(String),
    /// Output from another tool, shown exactly as it came.
    Raw(String),
}

#[derive(Debug)]
pub struct Problem {
    title: &'static str,
    context: Option<String>,
    blocks: Vec<Block>,
    hint: Option<String>,
    exit_code: u8,
}

impl Problem {
    /// `title` is shouted in the header bar, so keep it to a few words.
    pub fn new(title: &'static str) -> Self {
        Self { title, context: None, blocks: Vec::new(), hint: None, exit_code: 1 }
    }

    /// Shown at the right end of the header bar: usually the command or file.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn text(mut self, paragraph: impl Into<String>) -> Self {
        self.blocks.push(Block::Text(paragraph.into()));
        self
    }

    pub fn code(mut self, snippet: impl Into<String>) -> Self {
        self.blocks.push(Block::Code(snippet.into()));
        self
    }

    pub fn raw(mut self, output: impl Into<String>) -> Self {
        self.blocks.push(Block::Raw(output.into()));
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Marks this as a mistake in how `func` was called (exit status 2).
    pub fn usage(mut self) -> Self {
        self.exit_code = 2;
        self
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }

    pub fn render(&self, color: bool) -> String {
        let context = self.context.as_deref().unwrap_or("");
        let mut bar = format!("-- {} ", self.title);
        let used = bar.chars().count() + if context.is_empty() { 0 } else { context.chars().count() + 1 };
        bar.push_str(&"-".repeat(WIDTH.saturating_sub(used).max(3)));
        if !context.is_empty() {
            bar.push(' ');
            bar.push_str(context);
        }

        let mut out = term::paint(&bar, Style::Cyan, color);
        out.push_str("\n\n");

        for block in &self.blocks {
            match block {
                Block::Text(text) => {
                    out.push_str(text);
                    out.push_str("\n\n");
                }
                Block::Code(code) => {
                    for line in code.lines() {
                        out.push_str("    ");
                        out.push_str(line);
                        out.push('\n');
                    }
                    out.push('\n');
                }
                Block::Raw(raw) => {
                    out.push_str(raw.trim_end());
                    out.push_str("\n\n");
                }
            }
        }

        if let Some(hint) = &self.hint {
            out.push_str(&term::paint("Hint:", Style::Bold, color));
            out.push(' ');
            out.push_str(hint);
            out.push_str("\n\n");
        }
        out
    }

    /// Writes to stderr. A closed pipe (`func x 2>&1 | head`) is not worth a panic.
    pub fn print(&self) {
        use std::io::Write;
        let _ = std::io::stderr().write_all(self.render(term::stderr_color()).as_bytes());
    }
}

/// A file-system operation failed. Says what we were doing, to which path,
/// and what the operating system answered.
pub fn io(action: &str, path: &Path, error: &std::io::Error) -> Problem {
    Problem::new("FILE SYSTEM ERROR")
        .text(format!("I could not {action}:"))
        .code(path.display().to_string())
        .text(format!("The operating system said: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_title_context_body_and_hint() {
        let text = Problem::new("UNKNOWN SCRIPT")
            .context("func run helo")
            .text("I could not find a script named `helo`.")
            .code("hello")
            .hint("Create it with `func edit helo`.")
            .render(false);

        assert!(text.starts_with("-- UNKNOWN SCRIPT "));
        assert!(text.lines().next().unwrap().ends_with(" func run helo"));
        assert!(text.contains("\n    hello\n"));
        assert!(text.contains("Hint: Create it with `func edit helo`."));
    }

    #[test]
    fn header_bar_is_eighty_columns() {
        let text = Problem::new("X").context("ctx").render(false);
        assert_eq!(text.lines().next().unwrap().chars().count(), WIDTH);
    }

    #[test]
    fn usage_errors_exit_with_two() {
        assert_eq!(Problem::new("X").usage().exit_code(), 2);
        assert_eq!(Problem::new("X").exit_code(), 1);
    }

    #[test]
    fn no_escape_codes_without_color() {
        assert!(!Problem::new("X").hint("y").render(false).contains('\x1b'));
    }
}
