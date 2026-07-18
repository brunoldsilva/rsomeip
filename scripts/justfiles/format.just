# Commands for enforcing a consistent style.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Format all source code.
all:
    # Format Rust code.
    just format rust
    # Format shell scripts.
    just format shellscript
    # Format just files.
    just format justfile

alias a := all

# Format Rust code.
rust:
    cargo fmt

alias rs := rust

# Format shell scripts.
shellscript:
    shfmt --indent 4 --write .

alias sh := shellscript

# Format just files.
justfile:
    just --fmt
