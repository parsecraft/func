# Security

## What func is, and what it trusts

func compiles and runs Rust code **with your full user privileges**: that is its entire job. Only run scripts you wrote or have read, exactly as you would with a shell script. In particular:

- `// dep:` lines make Cargo download and build crates, and crates can run build scripts. Treat them like any dependency you add to a project.
- `func run ./file.rs` runs the file you point at.
- func never fetches scripts from the network on its own, and it keeps everything under your own directories (`~/.local/share/func`, `~/.cache/func`, and `~/.local/bin` for `install`).

## Reporting a vulnerability

Please report privately, not in a public issue. Use GitHub's private reporting: https://github.com/parsecraft/func/security/advisories/new

Include what you found, how to reproduce it, and the impact you expect. You can expect a first reply within a week. Fixes ship in a new release, and reporters are credited unless they prefer otherwise.

Examples of what counts: a script name or path that escapes the script directory, `install` overwriting a program it should not, or the installer scripts doing something other than what they print.

## Supported versions

Only the latest release receives fixes.
