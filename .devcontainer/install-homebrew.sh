#!/usr/bin/env bash

# Bash Strict Mode.
set -euo pipefail
IFS=$'\n\t'

# Path where Homebrew is installed.
BREW_PREFIX=/home/linuxbrew/.linuxbrew
# Name of the Dev Container user.
USERNAME=vscode

# Installs Homebrew and related dev dependencies.
function main {
    # Check for missing arguments.
    if [[ "$#" -eq 0 ]]; then
        echo "error: missing arguments" >&2
        exit 1
    fi

    # Parse command-line arguments.
    while [[ "$#" -gt 0 ]]; do
        case "$1" in
        homebrew)
            install-homebrew
            shift
            ;;
        deps)
            install-deps
            shift
            ;;
        all)
            install-homebrew
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

function install-homebrew {
    # Install Homebrew.
    NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Update the environment.
    update-env
}

function install-deps {
    # Install dev dependencies.
    "${BREW_PREFIX}/bin/brew" install --no-ask \
        just \
        markdownlint-cli \
        shellcheck \
        shfmt \
        vale
}

function update-env {
    # Update PATH to include Homebrew binaries.
    tee --append /etc/bash.bashrc <<EOF
if [[ "\${PATH}" != *"${BREW_PREFIX}/sbin"* ]]; then
    export PATH="${BREW_PREFIX}/sbin:\${PATH}";
fi
if [[ "\${PATH}" != *"${BREW_PREFIX}/bin"* ]]; then
    export PATH="${BREW_PREFIX}/bin:\${PATH}";
fi
EOF

    # Change ownership to the Dev Container user.
    chown -R "$USERNAME" "$BREW_PREFIX"
}

# Entrypoint of the script.
main "$@"
