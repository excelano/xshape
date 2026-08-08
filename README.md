# xshape — reshape for tabular data

xshape changes the *geometry* of a single table — which axis holds which cells — and never changes, filters, or aggregates a value. Pivot a long key/value table into a matrix, unpivot a wide export back to long so it can be queried, split one column into several, explode a delimited cell into rows, merge several columns into one, or transpose a table that arrived sideways. Every cell that goes in comes out somewhere else, unchanged.

**Project page:** [https://excelano.com/xshape/](https://excelano.com/xshape/)

```text
$ xshape unpivot --cols '[fy2020]:[fy2026]' --into fiscal_year,spend contracts.csv

contract_id,vendor,fiscal_year,spend
0000000000000000000002256,IDEMIA Identity & Security USA LLC,fy2020,0.00
0000000000000000000002256,IDEMIA Identity & Security USA LLC,fy2021,0.00
0000000000000000000002256,IDEMIA Identity & Security USA LLC,fy2022,0.00
...

$ xshape explode --col '[application_names]' --sep '; ' contracts.csv
# one row per application, every other field repeated — 9,217 pieces → 9,217 rows
```

## Why

Every messy-CSV workflow eventually hits the same wall: the data is *correct* but *oriented wrong* for the next step. A wide fiscal-year export has to go long before SQL can group it. A long event log has to spread into a matrix before a human can read it. A cell holds three application names glued by a semicolon and each needs its own row. The sheet arrived transposed. None of this changes what the data says — it only changes which axis holds what.

Until now that job had no home in a focused tool, so the reach was always for a general reshaper with a hundred other features attached — Miller, tidyr, a pandas `melt`/`pivot`/`explode` script. xshape is that one job, done with a bright boundary: it moves cells between the row and column axes, requires you to state your separator (it never guesses one), and **errors rather than aggregate** when two cells would collide — because the moment a reshape needs a `SUM`, it has become a query.

## The family

xshape is the reshaping verb in a family of small tools for messy tabular data, split by *what changes*:

- **[xray](https://github.com/excelano/xray)** observes — profiles the file, reports hazards, changes nothing.
- **[xled](https://github.com/excelano/xled)** edits *cell values* in place (sed and awk for tables).
- **xshape** changes the *shape* of the grid, and never touches the values.
- **[xql](https://github.com/excelano/xql)** queries the row *set* — filter, aggregate, group, join.

The line that keeps xshape focused: it moves cells between axes and refuses to aggregate. An aggregating pivot (`pivot … SUM`) is a query — that belongs to xql or DuckDB. xshape shares xled's column-addressing dialect (letters, `[bracketed names]`, ranges), so the family reads as one language with different verbs.

## Install

Every install line below ends with `xshape --install-skill`. That installs the [Claude Code skill](#use-it-from-claude-code) alongside the binary, which is the one step people reliably skipped when it lived further down the page. Drop it if you do not use Claude Code — the CLI itself does not need it.

### Debian and Ubuntu

Add the [Excelano apt repository](https://excelano.com/apt/) once:

```sh
curl -fsSL https://excelano.com/apt/setup.sh | sudo sh
```

Then:

```sh
sudo apt install xshape && xshape --install-skill
```

Both amd64 and arm64 packages ship with every release.

### Homebrew

```sh
brew install excelano/tap/xshape && xshape --install-skill
```

### crates.io

```sh
cargo install xshape && xshape --install-skill
```

### Windows

With [WinGet](https://learn.microsoft.com/windows/package-manager/), so `winget upgrade` keeps it current:

```powershell
winget install Excelano.xshape
xshape --install-skill
```

Or run the standalone installer in PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/excelano/xshape/releases/latest/download/xshape-installer.ps1 | iex"
```

### Curl (any Linux or macOS)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/excelano/xshape/main/install.sh | sh
```

To remove it: swap `install.sh` for `uninstall.sh` in that line.

## Usage

```sh
xshape <verb> [options] file.csv     # reshape; result to stdout
… | xshape <verb> [options]          # reshape piped stdin
xshape <verb> -i [options] file.csv  # write the result back to the file (like sed -i)
```

The reshaped table goes to **stdout by default** — the whole new grid in view before you commit. `-i`/`--in-place[=.bak]` writes it back.

| Verb | Direction | Example |
|---|---|---|
| `unpivot` | wide → long | `xshape unpivot --cols '[q1]:[q4]' --into quarter,value f.csv` |
| `pivot` | long → wide | `xshape pivot --names-from '[metric]' --values-from '[value]' f.csv` |
| `split` | one col → several | `xshape split --col '[name]' --sep ', ' --into last,first f.csv` |
| `explode` | cell → rows | `xshape explode --col '[tags]' --sep '; ' f.csv` |
| `merge` | several → one | `xshape merge --cols '[first],[last]' --sep ' ' --into name f.csv` |
| `transpose` | swap axes | `xshape transpose f.csv` |

Columns are addressed like xled: a letter (`C`, `AF`), a bracketed header name (`[first name]`), and — for the set-taking verbs — ranges (`E:K`) and comma lists (`[id],D:E`). `split`/`explode`/`merge` require an explicit `--sep` and split on it literally (spaces included); nothing is ever trimmed, dropped, or aggregated. `pivot` errors on a collision and points you to xql.

## Use it from Claude Code

xshape was built for AI coding agents as much as for people, so the repo ships an official [Claude Code](https://docs.claude.com/en/docs/claude-code) skill under [`skills/xshape/`](skills/xshape/). It teaches an agent the reshape verbs, the shared addressing dialect, the boundary rules (geometry only, explicit separators, no silent drop or combine), and — with a tidyr/pandas/Miller translation table — how to reach for `xshape` instead of a pandas `melt` the moment a reshape is needed. The binary installs it:

```sh
xshape --install-skill
```

That writes `~/.claude/skills/xshape/` and stamps in the version it came from, so a later run reports whether the skill has fallen behind the binary rather than leaving you to notice. It is safe to re-run: an unchanged skill reports `already current` and nothing is written. `xshape --uninstall-skill` removes it. Restart Claude Code afterwards, since skills are discovered at session start.

The skill is compiled into the binary, so this works the same however you installed xshape — apt, Homebrew, cargo, the curl one-liner, or a build from source.

## License

MIT. Authored by David M. Anderson, with AI assistance.
