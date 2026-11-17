//! Commands that manage the store: `list`, `show`, `path`, `rename`,
//! `remove`, `clean`.

use super::{Ctx, require_stored};
use crate::report::{self, Problem};
use crate::script::describe;
use crate::store;
use crate::term::{self, Style};
use std::fs;

pub fn list(ctx: &Ctx, names_only: bool) -> Result<u8, Problem> {
    let names = ctx.store.names()?;

    if names_only {
        names.iter().for_each(|name| term::write_stdout(&format!("{name}\n")));
        return Ok(0);
    }
    if names.is_empty() {
        eprintln!("You have no scripts yet. Create one with `func edit <name>`.");
        return Ok(0);
    }

    let color = term::stdout_color();
    let width = names.iter().map(String::len).max().unwrap_or(0);
    let mut out = String::new();
    for name in &names {
        let description = fs::read_to_string(ctx.store.path(name)).ok().and_then(|src| describe(&src));
        let label = term::paint(&format!("{name:<width$}"), Style::Bold, color);
        match description {
            Some(text) => out.push_str(&format!("{label}  {text}\n")),
            None => out.push_str(&format!("{}\n", label.trim_end())),
        }
    }
    term::write_stdout(&out);
    Ok(0)
}

pub fn show(ctx: &Ctx, name: &str) -> Result<u8, Problem> {
    require_stored(ctx, name)?;
    let path = ctx.store.path(name);
    let source = fs::read_to_string(&path).map_err(|e| report::io("read the script", &path, &e))?;
    term::write_stdout(&source);
    Ok(0)
}

pub fn path(ctx: &Ctx, name: Option<&str>) -> Result<u8, Problem> {
    let shown = match name {
        Some(name) => {
            require_stored(ctx, name)?;
            ctx.store.path(name)
        }
        None => ctx.store.dir().to_path_buf(),
    };
    term::write_stdout(&format!("{}\n", shown.display()));
    Ok(0)
}

pub fn rename(ctx: &Ctx, from: &str, to: &str) -> Result<u8, Problem> {
    require_stored(ctx, from)?;
    store::validate_name(to)?;
    if ctx.store.exists(to) {
        return Err(Problem::new("NAME TAKEN")
            .context(format!("func rename {from} {to}"))
            .text(format!("A script called `{to}` already exists, and I will not overwrite it."))
            .hint(format!("Remove it first with `func rm {to}`, or choose another name.")));
    }
    ctx.store.rename(from, to)?;
    eprintln!("Renamed `{from}` to `{to}`.");
    Ok(0)
}

pub fn remove(ctx: &Ctx, names: &[String], yes: bool) -> Result<u8, Problem> {
    // Check everything first, so a typo in the last name cannot leave a half-done job.
    for name in names {
        require_stored(ctx, name)?;
    }

    if !yes {
        if !term::stdin_is_terminal() {
            return Err(Problem::new("NEEDS CONFIRMATION")
                .text("Deleting scripts cannot be undone, and there is no terminal to ask you in.")
                .hint("Pass -y to confirm: `func rm -y <name>`."));
        }
        let question =
            format!("Delete {}?", names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", "));
        if !term::confirm(&question, false) {
            eprintln!("Nothing deleted.");
            return Ok(1);
        }
    }

    for name in names {
        ctx.store.remove(name)?;
    }
    eprintln!("Deleted {}.", names.join(", "));
    Ok(0)
}

pub fn clean(ctx: &Ctx) -> Result<u8, Problem> {
    let cache = ctx.builder.cache_dir();
    if !cache.exists() {
        eprintln!("The cache is already empty.");
        return Ok(0);
    }
    fs::remove_dir_all(cache).map_err(|e| report::io("delete the cache", cache, &e))?;
    eprintln!("Deleted cached builds. Your scripts were not touched.");
    Ok(0)
}
