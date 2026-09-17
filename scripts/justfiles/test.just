# Commands for testing the source code.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Run all test suites.
all:
    # Run Rust test suites.
    just test rust

alias a := all

# Run Rust test suites.
rust suite="all":
    # Run unit test suite.
    if test "{{ suite }}" = "all" || test "{{ suite }}" = "unit"; then \
        cargo hack --feature-powerset test; \
    fi
    # Run mutation tests.
    if test "{{ suite }}" = "all" || test "{{ suite }}" = "mutants"; then \
        cargo mutants --all-features --output target/ --jobs $(( ($(nproc) + 1) / 4 )); \
    fi
    # Measure test code coverage.
    if test "{{ suite }}" = "all" || test "{{ suite }}" = "coverage"; then \
        cargo +nightly llvm-cov clean; \
        cargo +nightly llvm-cov test --no-report --all-features --tests --benches; \
        cargo +nightly llvm-cov test --no-report --all-features --doctests; \
        cargo +nightly llvm-cov report --doctests --fail-under-lines 80 --html; \
    fi

alias rs := rust
