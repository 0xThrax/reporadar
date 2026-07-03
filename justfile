# Task runner contract. `just ci` exactly mirrors GitHub Actions CI.

default:
    @just --list

# Run the app against a directory (default: ~/Work)
run root="~/Work":
    cargo run -- {{root}}

test:
    cargo nextest run

# The pre-merge gate. Run before every PR.
ci: fmt-check clippy test deny machete

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

deny:
    cargo deny check

machete:
    cargo machete

cov:
    cargo llvm-cov nextest --html
