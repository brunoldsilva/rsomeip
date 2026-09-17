#!/usr/bin/env bash

# Bash Strict Mode.
set -euo pipefail
IFS=$'\n\t'

# Installs and updates the Rust toolchain and other dependencies.
function main {
    # Check for missing arguments.
    if [[ "$#" -eq 0 ]]; then
        echo "error: missing arguments" >&2
        exit 1
    fi

    # Parse command-line arguments.
    while [[ "$#" -gt 0 ]]; do
        case "$1" in
        rustup)
            install-rustup
            shift
            ;;
        toolchain)
            install-toolchain
            shift
            ;;
        targets)
            install-targets
            shift
            ;;
        deps)
            install-deps
            shift
            ;;
        all)
            install-rustup
            install-toolchain
            install-targets
            install-deps
            shift
            ;;
        *)
            echo "error: unknown command: '$1'" >&2
            exit 1
            ;;
        esac
    done
}

function install-rustup {
    # Download rustup from official sources.
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s
    # Reload the environment.
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
}

function install-toolchain {
    # Install the toolchains.
    rustup toolchain install --profile default stable 1.85
}

function install-targets {
    # A `no_std` target.
    rustup target install thumbv7m-none-eabi
}

function install-deps {
    # Install cargo-binstall to download other dependencies.
    cargo install --locked cargo-binstall
    # Download Cargo dependencies.
    cargo binstall --locked --no-confirm --disable-telemetry \
        cargo-deny \
        cargo-hack \
        cargo-llvm-cov \
        cargo-mutants \
        cargo-shear
}

# Entrypoint of the script.
main "$@"
