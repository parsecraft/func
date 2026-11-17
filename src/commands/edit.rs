//! `func edit`: the funced of Rust scripts.

use super::Ctx;
use crate::report::{self, Problem};
use crate::script::{self, Origin, Script};
use crate::{editor, store, term};
use std::fs;
use std::path::Path;

pub fn edit(ctx: &Ctx, name: &str) -> Result<u8, Problem> {
    store::validate_name(name)?;

    let path = ctx.store.path(name);
    let existed = path.is_file();
    let original = if existed {
        read(&path)?
    } else {
        let template = script::template(name);
        ctx.store.write(name, &template)?;
        template
    };

    loop {
        editor::open(&path)?;
        let current = read(&path)?;

        // Leaving a brand-new script untouched means "never mind".
        if !existed && (current.trim().is_empty() || current == original) {
            ctx.store.remove(name)?;
            eprintln!("Nothing written, so `{name}` was not saved.");
            return Ok(0);
        }
        if existed && current == original {
            eprintln!("No changes to `{name}`.");
            return Ok(0);
        }

        let checked =
            Script::from_source(name, &path, current, Origin::Store).and_then(|s| ctx.builder.check(&s));
        match checked {
            Ok(()) => {
                eprintln!("Saved `{name}`. Run it with `func run {name}`.");
                return Ok(0);
            }
            Err(problem) => {
                problem.print();
                // The file is already saved: errors never cost anyone their work.
                if !term::stdin_is_terminal() || !term::confirm("Reopen the editor?", true) {
                    eprintln!("Your changes to `{name}` are saved. Continue later with `func edit {name}`.");
                    return Ok(1);
                }
            }
        }
    }
}

fn read(path: &Path) -> Result<String, Problem> {
    fs::read_to_string(path).map_err(|e| report::io("read the script", path, &e))
}
