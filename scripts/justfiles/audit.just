# Commands for auditing dependencies.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Audit all dependencies.
all:
    # Audit Rust dependencies.
    just audit rust

alias a := all

# Audit Rust dependencies.
rust:
    # Check for unused dependencies.
    cargo shear
    # Check for advisories, incompatible licenses and banned depencencies.
    cargo deny check
    # Check if dependencies have been properly vetted.
    cargo vet

alias rs := rust
