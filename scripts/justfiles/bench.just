# Commands for benchmarking source code.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Run all benchmarks.
all:
    # Run Rust benchmarks.
    just bench rust

alias a := all

# Run Rust benchmarks.
rust *ARGS:
    # Build the benchmarks first.
    cargo build --benches --release
    # Do a quick sanity check.
    cargo test --benches
    # Wait for the environment to stabilize before running the benchmarks.
    sleep 30
    # Run the benchmark tests on an isolated core.
    taskset -c 8 cargo bench -- {{ ARGS }}

alias rs := rust
