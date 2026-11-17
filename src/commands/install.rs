//! `func install`: the funcsave of Rust scripts. Scripts already persist in
//! the store; installing turns one into a real program on your PATH.
//!
//! func keeps a small ledger (`installed`, one name per line) of what it
//! put in the bin directory, so it never overwrites a program it did not
//! create unless asked to.

use super::{Ctx, load_target};
use crate::build::Profile;
use crate::report::{self, Problem};
use std::fs;
use std::path::Path;

pub fn install(ctx: &Ctx, target: &str, force: bool) -> Result<u8, Problem> {
    let script = load_target(ctx, target)?;
    let destination = ctx.dirs.bin.join(&script.name);
    let ledger = ctx.dirs.data.join("installed");
    let mut installed = read_ledger(&ledger);

    // Check before the (possibly long) build, not after.
    let ours = installed.contains(&script.name);
    if destination.exists() && !ours && !force {
        return Err(Problem::new("WON'T OVERWRITE")
            .context(format!("func install {}", script.name))
            .text("A program with this name already exists, and I did not put it there:")
            .code(destination.display().to_string())
            .hint("Pick another script name, or pass --force if you really want to replace it."));
    }

    let built = ctx.builder.compile(&script, Profile::Release)?;

    fs::create_dir_all(&ctx.dirs.bin)
        .map_err(|e| report::io("create the bin directory", &ctx.dirs.bin, &e))?;
    // Copy then rename: a running copy of the old version is replaced atomically.
    let partial = ctx.dirs.bin.join(format!(".{}.partial", script.name));
    fs::copy(&built, &partial)
        .map_err(|e| report::io("copy the build to the bin directory", &partial, &e))?;
    fs::rename(&partial, &destination).map_err(|e| report::io("install the program", &destination, &e))?;

    if !ours {
        installed.push(script.name.clone());
        write_ledger(&ledger, &installed)?;
    }

    eprintln!("Installed {}", destination.display());
    if !is_on_path(&ctx.dirs.bin) {
        eprintln!(
            "Note: {} is not on your PATH yet. Add it to your shell profile to run `{}` anywhere.",
            ctx.dirs.bin.display(),
            script.name
        );
    }
    Ok(0)
}

fn read_ledger(path: &Path) -> Vec<String> {
    fs::read_to_string(path).map(|text| text.lines().map(str::to_string).collect()).unwrap_or_default()
}

fn write_ledger(path: &Path, names: &[String]) -> Result<(), Problem> {
    let mut text = names.join("\n");
    text.push('\n');
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| report::io("create the data directory", dir, &e))?;
    }
    fs::write(path, text).map_err(|e| report::io("record the installed program", path, &e))
}

fn is_on_path(dir: &Path) -> bool {
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|entry| entry == dir))
}
