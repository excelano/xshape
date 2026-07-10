#!/bin/sh
# xshape — installer shim
#
# Delegates to the cargo-dist-generated installer for the latest release.
# This exists so the install and uninstall one-liners share a URL shape:
#
#     curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/excelano/xshape/main/install.sh | sh
#     curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/excelano/xshape/main/uninstall.sh | sh

set -eu

curl --proto '=https' --tlsv1.2 -LsSf \
    https://github.com/excelano/xshape/releases/latest/download/xshape-installer.sh | sh
