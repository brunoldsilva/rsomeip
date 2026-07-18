# Commands for running executables.

# Enable `no-cd`, otherwise these commands will always run in this folder.
set no-cd

# Example from rsomeip-proto.
request-sample *ARGS:
    cargo run --example request-sample -- {{ARGS}}

# Example from rsomeip-proto.
response-sample *ARGS:
    cargo run --example response-sample -- {{ARGS}}
