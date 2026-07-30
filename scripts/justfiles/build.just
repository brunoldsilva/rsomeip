# Commands for building binary targets.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Minimum Supported Rust Version (MSRV).
MSRV := "1.85"

# Compile all binary targets.
all:
    # Compile Rust binary targets.
    just build rust

alias a := all

# Compile Rust binary targets.
rust:
    # Compile binary targets with each and every feature.
    cargo hack --feature-powerset build --release --lib --examples
    # Compile binary targets using the Minimum Supported Rust Version.
    cargo +{{ MSRV }} hack --feature-powerset build --release --lib --examples
    # Compile binary targets against a `no_std` target.
    cargo hack --feature-powerset  --skip std,default build --release --lib --examples --target thumbv7m-none-eabi

alias rs := rust

# Build project documentation.
docs:
    # Build Rust documentation.
    cargo doc --no-deps --all-features

alias d := docs
