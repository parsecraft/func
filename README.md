# func

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Write, test and run Rust scripts on the fly. No project, no `Cargo.toml`, no copies of your code.

Inspired by fish's `funced`, `function` and `funcsave`: you edit a function by name, it works immediately, and it is saved the moment you close the editor.

```console
$ func edit hello        # opens $EDITOR, type-checks when you close it
Saved `hello`. Run it with `func run hello`.

$ func run hello world   # compiles once, then runs instantly (about 3 ms when cached)
hello world

$ func test hello        # runs the script's own #[test] functions
$ func install hello     # optimized build on your PATH, like funcsave
```

No dependencies, one small binary, and startup is a hash lookup followed by `exec`.

## Install

func builds your scripts with Rust, so you need [Rust](https://rustup.rs) (and `git`) either way.

**One line:**

```sh
curl -fsSL https://raw.githubusercontent.com/parsecraft/func/main/install.sh | sh
```

It fetches the source, builds an optimized binary, installs it to `~/.local/bin` and adds completions for the shells you have. Read the script first if you like; it prints everything it touches. For another location: `... | sh -s -- --prefix /usr/local`.

**From a checkout:**

```sh
git clone https://github.com/parsecraft/func
cd func
./install.sh        # or: make install
```

**With Cargo:** `cargo install --git https://github.com/parsecraft/func` (no completions; see below).

Remove it again with `./uninstall.sh` (`--purge` also deletes your scripts).

### Completions

`install.sh` does this for you. To do it yourself:

| shell | setup |
|-------|-------|
| bash  | `eval "$(func completions bash)"` in `~/.bashrc` |
| zsh   | `func completions zsh > ~/.zfunc/_func`, with `fpath=(~/.zfunc $fpath)` before `compinit` |
| fish  | `func completions fish > ~/.config/fish/completions/func.fish` |

`func <Tab>` completes commands, and only commands, so hundreds of scripts never clutter it. Script names complete where a script is expected (`func run <Tab>`, `func edit <Tab>`, `func rm <Tab>`), live: a script you created a second ago is already there.

## The 20% that covers 99%

| you want to | run |
|---|---|
| write or change a script | `func edit name` |
| run it | `func run name [args]` (short: `func r name`) |
| run it optimized | `func run -r name [args]` |
| test it | `func test name [filter]` |
| check it compiles, fast | `func check name` |
| try one line | `func eval 'println!("{}", 6 * 7);'` |
| put it on your PATH | `func install name` |
| see what you have | `func list` |
| read, copy or locate it | `func show name`, `func show name > copy.rs`, `func path name` |
| rename or delete it | `func rename old new`, `func rm name` |
| free disk space | `func clean` |

Run `func help` for the list, or `func help <command>` for details. Short forms: `new`/`e`, `r`, `t`, `ls`, `cat`, `mv`, `rm`.

### Commands and scripts never mix

Scripts run **only** through `func run`. There is no bare `func hello`, so a command is always a command, and a script is always something you named on purpose. A script called `edit2`, or even `edit` or `list`, can never be mistaken for a command (`func edit` always edits; `func run edit` always runs your script). A mistyped command such as `func edti` gets a "did you mean `edit`?" error instead of being silently treated as a script.

Want a plain `hello` in your shell? `func install hello` puts a real program on your PATH.

### Without copies

A script lives in exactly one place: `~/.local/share/func/scripts/<name>.rs`. Nothing is ever duplicated.

- **Saved scripts** are addressed by name. `func show` and `func path` give you the file when another tool needs it.
- **Files anywhere** run in place: `func run ./tool.rs`. They are not imported. Edit them in your own editor and the next run picks the change up.
- **Builds** are cached by content, so an unchanged script never recompiles, and a changed one compiles once.
- **Dependencies** share one build directory. A crate compiles once for all your scripts, not once per script.

### Using crates

Put directives in the comment block at the top of the script. They are ordinary comments, so the file stays valid Rust for your editor and for `rustfmt`.

```rust
//! Counts the words in a file.
// dep: regex = "1"
// edition: 2021

use regex::Regex;

fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    println!("{}", Regex::new(r"\w+").unwrap().find_iter(&text).count());
}
```

| line | meaning |
|---|---|
| `//! text` | description, shown by `func list` |
| `// dep: name = "version"` | a crate; also `name = { version = "1", features = ["x"] }`, `name = 1.2` or a bare `name`. Repeat the line for more |
| `// edition: 2021` | Rust edition. Default: 2024 |

Scripts with `dep` lines build through Cargo (the first build downloads, later runs are cached). Scripts without them use `rustc` directly, which is the fastest path.

### Editing

`func edit name` opens `$VISUAL`, then `$EDITOR`, then `nano` or `vi`. When you close it, func type-checks the script. If there are errors you are offered the editor again, with your work intact. Closing a new script without changing the template saves nothing.

### Arguments belong to your script

Everything after the script name goes to the script, flags included: `func run hello -r --help` passes both flags to `hello`. Put func's own flags before the name: `func run -r hello`.

### Exit codes

A script's own exit code passes straight through, because func replaces itself with your program (`exec`) on Unix. For func's own errors: `1` is a failure (compile error, unknown script), `2` is a mistake in how func was called.

### Examples

The [examples/](examples) directory has small scripts to try:

```sh
func run examples/hello.rs Morocco
func run examples/fizzbuzz.rs 15
func test examples/fizzbuzz.rs
func run examples/wordcount.rs README.md      # uses the regex crate
```

## Error messages

Errors say what happened, show the relevant thing, and suggest the next step. In the style of Elm:

```text
-- UNKNOWN COMMAND --------------------------------------------------- func edti

I do not have a command called `edti`.

Did you mean one of these?

    edit

Hint: To run a script called `edti`, use `func run edti`. All commands: `func help`.
```

```text
-- UNKNOWN SCRIPT --------------------------------------------------------- helo

I could not find a script named `helo`.

Did you mean one of these scripts?

    hello

Hint: Create it with `func edit helo`, or see what you have with `func list`.
```

```text
-- BAD HEADER LINE ------------------------------------------------------- hdr:1

I could not understand this line at the top of your script:

    // deps: regex = "1"

`deps` is not a directive. Dependencies are declared with `dep`.

Hint: It should look like `// dep: regex = "1"`.
```

Compiler errors come through unchanged inside a frame that names the script. Color is on only in a terminal and respects `NO_COLOR`.

## Configuration

Almost none, on purpose.

| variable | effect |
|---|---|
| `VISUAL`, `EDITOR` | which editor `func edit` opens |
| `FUNC_HOME` | keep everything (scripts, cache, bin) under one directory. Useful for sandboxes and tests |
| `FUNC_BIN` | where `func install` puts programs. Default `~/.local/bin` |
| `XDG_DATA_HOME`, `XDG_CACHE_HOME` | standard locations for scripts and cache |
| `NO_COLOR` | disable color |

## Development

```sh
make test         # unit tests, plus end-to-end tests that run the real binary
make lint         # cargo fmt --check and clippy
make help         # every task
```

The code is organized so each file has one job; see [docs/DESIGN.md](docs/DESIGN.md) for the map, the decisions behind it, and how to add a command.

If you change a command or flag, regenerate the committed completion files (a test fails until you do):

```sh
make completions
```

## Contributing

Issues and pull requests are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md); security reports go through [SECURITY.md](SECURITY.md). Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## License

MIT. See [LICENSE](LICENSE).
