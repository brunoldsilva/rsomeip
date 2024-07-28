#!/usr/bin/env bash

# Unofficial Bash Strict Mode.
set -euo pipefail
IFS=$'\n\t'

# Paths to the generated coverage artifacts.
readonly COVERAGE_DIR="${COVERAGE_DIR:-/tmp/rsomeip/coverage}"
readonly COVERAGE_BUILD_DIR="${COVERAGE_BUILD_DIR:-${COVERAGE_DIR}/build}"
readonly COVERAGE_TEST_DIR="${COVERAGE_TEST_DIR:-${COVERAGE_DIR}/tests}"

# Entrypoint into the script.
function main() {
    install_prerequisites
    build_and_run_tests
    generate_report
}

# Installs the prerequisites to generate coverage data and reports.
function install_prerequisites() {
    echo 'coverage.rs: installing prerequisites'
    check_rustup
    install_llvm_tools
}

# Checks if rustup is installed.
function check_rustup() {
    if ! command -v rustup &> /dev/null; then
        echo 'coverage.rs: error: command rustup not found' >&2
        exit 1
    fi
}

# Installs the llvm-tools component using rustup.
function install_llvm_tools() {
    if ! rustup component add llvm-tools; then
        echo 'coverage.rs: error: failed to install llvm-tools' >&2
        exit 1
    fi
}

# Builds and runs the tests with instrumented code coverage enabled.
function build_and_run_tests() {
    build_tests
    run_tests
}

# Builds the tests with instrumented code coverage enabled.
function build_tests() {
    backup_folder "${COVERAGE_BUILD_DIR}"

    echo 'coverage.rs: building tests'
    export RUSTFLAGS="-Cinstrument-coverage"
    export LLVM_PROFILE_FILE="${COVERAGE_BUILD_DIR}/rsomeip-%p-%m.profraw"
    if ! cargo build --verbose; then
        echo 'coverage.rs: error: failed to build tests' >&2
        exit 1
    fi
}

# Runs the tests to generate raw coverage data.
function run_tests() {
    backup_folder "${COVERAGE_TEST_DIR}"

    echo 'coverage.rs: running tests'
    export LLVM_PROFILE_FILE="${COVERAGE_TEST_DIR}/rsomeip-%p-%m.profraw"
    if ! cargo test --verbose; then
        echo 'coverage.rs: error: failed to run tests' >&2
        exit 1
    fi
}

# Creates a backup of the given folder as "folder.old".
#
# Removes any previously created backups.
function backup_folder() {
    local folder="$1"
    rm --force --recursive "${folder}.old" || true
    mv --force "${folder}" "${folder}.old" || true
}

# Generates a report from the raw coverage data.
function generate_report() {
    echo 'coverage.rs: generating coverage report'
    if ! grcov "${COVERAGE_TEST_DIR}" --source-dir ./rsomeip/src --binary-path ./target/debug/ \
         --output-types html,lcov --branch --ignore-not-existing \
         --output-path ./target/debug/coverage/
    then
        echo 'coverage.rs: error: failed to generate coverage report' >&2
        exit 1
    fi
}

main "$@"
