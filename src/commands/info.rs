//! `help`, `--version` and `completions`: the commands that need no storage.

use crate::cli::{self, COMMANDS, Shell};
use crate::report::Problem;
use crate::term::{self, Style};
use crate::{completions, suggest};

const TAGLINE: &str = "Write, test and run Rust scripts on the fly.";

pub fn version() -> Result<u8, Problem> {
    term::write_stdout(&format!("func {}\n", env!("CARGO_PKG_VERSION")));
    Ok(0)
}

pub fn completions(shell: Shell) -> Result<u8, Problem> {
    term::write_stdout(&completions::generate(shell));
    Ok(0)
}

pub fn help(topic: Option<&str>) -> Result<u8, Problem> {
    let color = term::stdout_color();
    let text = match topic {
        None => overview(color),
        Some(name) => match cli::find(name) {
            Some(spec) => command_help(spec, color),
            None => return Err(unknown_topic(name)),
        },
    };
    term::write_stdout(&text);
    Ok(0)
}

fn overview(color: bool) -> String {
    let bold = |s: &str| term::paint(s, Style::Bold, color);
    let mut out = format!("{}\n\n{}\n    func <command> [args]\n\n", TAGLINE, bold("USAGE"));

    out.push_str(&bold("COMMANDS"));
    out.push('\n');
    for spec in COMMANDS {
        let aliases =
            if spec.aliases.is_empty() { String::new() } else { format!(" ({})", spec.aliases.join(", ")) };
        out.push_str(&format!("    {:<14}{}{}\n", spec.name, spec.summary, aliases));
    }

    out.push_str(&format!(
        "\n{}\n    func edit hello          write a script (opens your editor)\n    func run hello           run it (short: func r hello)\n    func test hello          run its #[test] functions\n    func install hello       put it on your PATH\n    func eval 'println!(\"{{}}\", 6 * 7);'\n\n{}\n    Add `// dep: regex = \"1\"` at the top of a script to use a crate.\n    Run `func help <command>` for details.\n",
        bold("EXAMPLES"),
        bold("TIPS"),
    ));
    out
}

fn command_help(spec: &cli::Spec, color: bool) -> String {
    let bold = |s: &str| term::paint(s, Style::Bold, color);
    let mut out = format!("{}\n\n{}\n    {}\n", spec.summary, bold("USAGE"), cli::usage_line(spec));
    if !spec.aliases.is_empty() {
        out.push_str(&format!("\n{}\n    {}\n", bold("ALIASES"), spec.aliases.join(", ")));
    }
    out.push_str(&format!("\n{}\n", bold("DETAILS")));
    for line in spec.long.lines() {
        out.push_str(&format!("    {line}\n"));
    }
    out
}

fn unknown_topic(name: &str) -> Problem {
    let mut problem = Problem::new("UNKNOWN COMMAND")
        .context(format!("func help {name}"))
        .text(format!("There is no command called `{name}`."));
    let close = suggest::closest(name, COMMANDS.iter().map(|spec| spec.name));
    if !close.is_empty() {
        problem = problem.text("Did you mean one of these?").code(close.join("\n"));
    }
    problem.hint("Run `func help` to see every command.").usage()
}
