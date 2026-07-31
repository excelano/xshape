# Releasing xshape

The release loop for a new version. Run it from a clean `main` with the working tree committed.

The crate, the command, the Homebrew formula, and the apt package are all **`xshape`** — one name everywhere (unlike xray, whose crate is the hyphenated `x-ray`). cargo-dist's tarballs and installer are named after it: `xshape-installer.sh`, `xshape-<target>.tar.xz`.

1. **Bump the version.** Edit `version` in `Cargo.toml` (e.g. `0.1.0` → `0.1.1`). Update `Cargo.lock` with a build (`cargo build`), run `cargo test`, commit.

2. **Tag and push.** `git tag v0.1.1 && git push origin main --tags`. The `v*` tag triggers cargo-dist (`.github/workflows/release.yml`), which builds the five platform tarballs, the shell/PowerShell installers, the Homebrew formula, and the checksums, then creates the GitHub Release.

3. **Build the .debs.** cargo-dist creates the release with the default `GITHUB_TOKEN`, and GitHub does **not** fire `release: published` for token-created releases (a documented anti-recursion safeguard). So `deb.yml` won't auto-run — dispatch it by hand:
   ```sh
   gh workflow run deb.yml -f tag=v0.1.1
   ```
   It builds amd64 + arm64 packages and uploads them to the release.

4. **crates.io publishes itself.** The `v*` tag also triggers `publish-crate.yml`, which runs `cargo publish` with the org-secret token — so crates.io is live within a minute of the push, no local step. Confirm it succeeded:
   ```sh
   gh run list --workflow=publish-crate.yml --limit 1
   ```
   Do **not** run `cargo publish` by hand — the pipeline beats you to it and you'll just get `already exists`. **Versions are immutable**: you can `cargo yank` a bad release to hide it from new dependency resolution, but never re-publish the same number. A fix is always a fresh version bump, never a re-push.

5. **Add the .debs to the Excelano apt repo.** Download the two `.deb`s from the release, then in `~/excelano-apt/`: `add-deb.sh` each one → `rebuild.sh` (GPG-signs) → `updatesite excelano.com.apt -y`. **Dry-run the rsync first** (`rsync … --delete -n`) and confirm zero deletions before the real push — the apt pool is a superset of live, and a stray `--delete` wipe is the standing hazard.

6. **Submit the winget manifest.** winget stores one manifest per version, so every release needs its own PR — there is no update in place. Run komac:
   ```sh
   komac update Excelano.xshape --version 0.1.1 \
     --urls https://github.com/excelano/xshape/releases/download/v0.1.1/xshape-x86_64-pc-windows-msvc.zip \
     --submit
   ```
   It downloads the asset, computes the `InstallerSha256`, generates the manifest from the previous version's, and opens the PR against `microsoft/winget-pkgs`. Drop `--submit` (or add `--dry-run`) to eyeball the manifest first.

   A **version update** to an already-merged package usually clears automated validation and merges with no human involved. A **new package** picks up the `New-Package` label and waits on a volunteer moderator, which runs to days. Two failures recur, both with recipes in `~/notes/build_release_gotchas.md`: `Validation-Defender-Error` (a Defender heuristic flags the unsigned cargo-dist binary — submit the false positive, never rebuild to appease it) and `Validation-Executable-Error` (validation runs the exe with no arguments and reports a non-zero exit; `xshape` with no subcommand exits 2 and can trip it).

   **A pushed `v*` tag is spent.** The merged manifest pins `InstallerSha256`, so deleting and re-cutting a tag swaps the release asset out from under it and breaks every install of that version. That is the same immutability rule step 4 states for crates.io, except nothing here refuses the second attempt — winget, apt, and the Homebrew formula all overwrite silently. If a release goes wrong after the tag is pushed, bump to the next number.

## First release checklist (v0.1.0 — the one-time org setup)

xshape is a **new repo**, so the shared org secrets and the tap need it added to their access lists *before* the first tag, or the tagged jobs fail at checkout:

- **`CRATES_IO_TOKEN`** — the org secret `publish-crate.yml` reads. If it is scoped to selected repos, add `excelano/xshape`. A verified crates.io email and a token with `publish-new` + `publish-update` scopes are required before the first publish. The crate name `xshape` is confirmed free.
- **`HOMEBREW_TAP_TOKEN`** — cargo-dist pushes the formula to `excelano/homebrew-tap`; that tap repo must exist and the token must be allowed to push to it. **Gotcha (bit xray v0.1.0):** if it is an *org* secret scoped to selected repos, a brand-new repo isn't on the list, so the `publish-homebrew-formula` job fails at checkout with `Input required and not supplied: token`. Add `excelano/xshape` to the secret's access list **before tagging**. If it is already tagged, don't re-run that job on the old tag — push the formula to the tap by hand for that release; the token fix takes effect on the next one.

## Notes

- **crates.io API needs a User-Agent.** Requests without one return empty. To verify a publish from a script: `curl -s -H "User-Agent: …" https://crates.io/api/v1/crates/xshape`.
- **docs.rs** rebuilds automatically on each publish — no action needed.
- The README, the landing page (`excelano.com/xshape`), and `SECURITY.md` reference the version implicitly via "latest"; none need a per-release edit.
