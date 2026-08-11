# Releasing xshape

The release loop lives in `~/notes/releasing.md` — the ordered steps, the apt
step, crates.io, the winget submission, the spent-tag rule, and the standing
facts about tokens and secrets. Failure recipes are in
`~/notes/build_release_gotchas.md`. This file carries what is true of xshape and
not of its siblings.

| | |
|---|---|
| Loop | cargo-dist |
| Version lives in | `version` in `Cargo.toml` |
| `apt-ship` argument | `xshape` |
| crate | `xshape` |
| winget package | `Excelano.xshape` |
| Windows asset | `xshape-x86_64-pc-windows-msvc.zip` |

**The crate, the command, the Homebrew formula, and the apt package are all
`xshape`** — one name everywhere, unlike xray, whose crate is the hyphenated
`x-ray`. cargo-dist's tarballs and installer are named after it:
`xshape-installer.sh`, `xshape-<target>.tar.xz`.

**The release builds** the five platform tarballs, the shell and PowerShell
installers, the Homebrew formula, and the checksums, then creates the GitHub
Release. The `.deb` packages come from the separately dispatched `deb.yml`.

**xshape trips `Validation-Executable-Error` by design.** Invoked with no
subcommand it exits 2 — clap's own "no subcommand given" — which winget's
bare-invocation sweep can report as a failure. Recipe in the gotchas file; do not
add a no-argument success path to appease it.
