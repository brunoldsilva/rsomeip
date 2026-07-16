# Configure a Rust development environment.

# Based on the current Debian stable.
# More info: <https://github.com/devcontainers/templates/tree/main/src/rust>
FROM mcr.microsoft.com/devcontainers/rust:trixie

# Install Homebrew to manage dev dependencies.
COPY ./install-homebrew.sh /tmp/install-homebrew.sh
RUN /tmp/install-homebrew.sh all

# Update the Rust toolchain to include the stable and MSRV versions.
COPY ./install-rust.sh /tmp/install-rust.sh
RUN /tmp/install-rust.sh toolchain deps
