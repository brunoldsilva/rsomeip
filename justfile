# Just command runner configuration.
#
# Provides commands to standardize project workflows.
#
# More info: <https://just.systems/man/en/>

# Commands for auditing depenencies.
mod audit "scripts/justfiles/audit.just"

# Commands for building binary targets.
mod build "scripts/justfiles/build.just"

# Commands for checking the correctness of source code.
mod check "scripts/justfiles/check.just"

# Commands for enforcing a consistent style.
mod format "scripts/justfiles/format.just"

# Commands for running executables.
mod run "scripts/justfiles/run.just"

# Commands for testing the source code.
mod test "scripts/justfiles/test.just"

# List available commands.
@_default:
    just --list

# Run the entire verification suite.
verify:
    just check
    just build
    just test

alias v := verify
