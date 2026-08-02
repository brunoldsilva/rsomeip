# Commands for checking the correctness of source code.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Perform static code analysis checks.
all:
    # Perform static analysis of Rust code.
    just check rust
    # Perform static analysis of Shell scripts.
    just check shellscript
    # Perform static analysis of documentation.
    just check docs

# Perform static analysis of Rust code.
rust:
    # Check if Rust code is correctly formatted.
    cargo fmt --check
    # Check if Clippy complains.
    cargo hack --feature-powerset clippy --no-deps --profile test -- --deny warnings
    # Check if docs are correct.
    RUSTDOCFLAGS="-Dwarnings" cargo hack --feature-powerset doc --no-deps --document-private-items

alias rs := rust

# Perform static analysis of Shell scripts.
shellscript:
    # Check if shell scripts are correctly formatted.
    shfmt --indent 4 --list .
    # Check if Shell scripts are correct.
    find . -type f -name '*.sh' -print0 | xargs --no-run-if-empty -0 shellcheck

alias sh := shellscript

# Perform static analysis of documentation files.
docs:
    # Check Markdown files.
    markdownlint --ignore-path .gitignore .
    # Check prose in documentation.
    vale --glob '{rsomeip*/{{{{src,examples,benches}/*.rs,*.md},docs/**/*.md,*.md}' .
