# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub Security Advisories at https://github.com/excelano/xshape/security/advisories/new. If you would rather not use GitHub, email david.anderson@excelano.com instead. I aim to respond within seven days.

Please do not open public issues for security problems.

## Supported versions

The latest 0.x release receives security fixes. Older versions are not supported.

## What xshape can access

xshape is a CLI that runs locally on your machine. It reads the CSV or DSV file you point it at (or standard input), reshapes it, and prints the result to standard output. By default it writes nothing: the reshaped table goes to stdout. The only path that touches a file is `-i`/`--in-place`, which you request explicitly — it overwrites the input file with the reshaped result, optionally first copying the original to `<file>.bak`. xshape makes no network calls of any kind, has no auth layer, and implements no administrative operations. It can only read and, with `-i`, write files your operating-system user already has access to.

## What xshape stores

xshape stores nothing beyond the output you ask for. There is no config directory, no history file, no cache, no telemetry, no analytics, and no remote logging. It reads a table, writes the reshaped table (to stdout, or to the file with `-i`), and exits.

## Verifying releases

Every GitHub release includes a `.sha256` file next to each archive listing its SHA-256 hash. Verify any download before running it:

    sha256sum xshape-x86_64-unknown-linux-gnu.tar.xz
    # compare against the value in xshape-x86_64-unknown-linux-gnu.tar.xz.sha256

Release artifacts are built by GitHub Actions from a tagged commit using the cargo-dist configuration in this repo (`dist-workspace.toml` and the generated `.github/workflows/release.yml`). The workflow and build configuration are public and auditable.
