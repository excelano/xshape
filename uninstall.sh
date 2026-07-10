#!/bin/sh
# xshape — uninstaller
#
# Removes the xshape binary installed by install.sh. xshape stores nothing else on
# disk (no config, no history, no cache), so this is the entire cleanup.
#
#     curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/excelano/xshape/main/uninstall.sh | sh

set -eu

if [ -n "${CARGO_HOME:-}" ]; then
    install_dir="$CARGO_HOME/bin"
else
    install_dir="$HOME/.cargo/bin"
fi

target="$install_dir/xshape"

if [ -e "$target" ]; then
    rm -f "$target"
    echo "Removed $target"
elif command -v xshape >/dev/null 2>&1; then
    found="$(command -v xshape)"
    echo "xshape is installed at $found, not the expected location ($target)."
    echo "Remove it manually if you want it gone."
    exit 1
else
    echo "xshape is not installed."
fi
