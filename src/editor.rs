//! Opens a file in the person's editor: `$VISUAL`, then `$EDITOR`, then
//! whatever common editor is installed. Arguments are split on whitespace,
//! so `EDITOR="code --wait"` works.

use crate::report::Problem;
use std::path::Path;
use std::process::Command;

const FALLBACKS: [&str; 4] = ["nano", "vim", "vi", "notepad"];

pub fn open(path: &Path) -> Result<(), Problem> {
    let argv = pick(
        std::env::var("VISUAL").ok(),
        std::env::var("EDITOR").ok(),
        |program| find_in_path(program).is_some(),
    )
    .ok_or_else(|| {
        Problem::new("NO EDITOR FOUND")
            .text("I need a text editor to open your script, but $VISUAL and $EDITOR are not set and I could not find nano or vi.")
            .hint("Tell me which one to use, for example `export EDITOR=nano`.")
    })?;

    let (program, args) = argv.split_first().expect("pick never returns an empty command");
    let status = Command::new(program).args(args).arg(path).status().map_err(|error| {
        Problem::new("EDITOR WON'T START")
            .text(format!("I tried to start `{program}`, but could not: {error}"))
            .hint("Check $VISUAL and $EDITOR, or set one, for example `export EDITOR=nano`.")
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(Problem::new("EDITOR FAILED")
            .text(format!("`{program}` exited with {status}, so I did not check or save anything."))
            .hint("Run the same command again when you are ready."))
    }
}

/// Chooses the editor command. Pure, so the order of preference is testable.
fn pick(
    visual: Option<String>,
    editor: Option<String>,
    installed: impl Fn(&str) -> bool,
) -> Option<Vec<String>> {
    let configured = [visual, editor]
        .into_iter()
        .flatten()
        .map(|value| value.split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .find(|argv| !argv.is_empty());

    configured.or_else(|| {
        FALLBACKS.iter().find(|program| installed(program)).map(|program| vec![(*program).to_string()])
    })
}

fn find_in_path(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(program)).find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_installed(_: &str) -> bool {
        false
    }

    #[test]
    fn visual_beats_editor() {
        let argv = pick(Some("code --wait".into()), Some("nano".into()), nothing_installed).unwrap();
        assert_eq!(argv, ["code", "--wait"]);
    }

    #[test]
    fn blank_settings_are_skipped() {
        let argv = pick(Some("  ".into()), Some("vim".into()), nothing_installed).unwrap();
        assert_eq!(argv, ["vim"]);
    }

    #[test]
    fn falls_back_to_an_installed_editor() {
        let argv = pick(None, None, |program| program == "vi").unwrap();
        assert_eq!(argv, ["vi"]);
    }

    #[test]
    fn no_editor_at_all_is_none() {
        assert!(pick(None, None, nothing_installed).is_none());
    }
}
