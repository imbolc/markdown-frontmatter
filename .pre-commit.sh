#!/usr/bin/env sh

set -eu

# Linking the script as the pre-commit hook
SCRIPT_PATH=$(realpath "$0")
HOOK_PATH=$(git rev-parse --git-dir)/hooks/pre-commit

if [ "$(realpath "$HOOK_PATH")" != "$SCRIPT_PATH" ]; then
    printf "Link this script as the git pre-commit hook to avoid further manual running? (y/N): "
    read -r link_hook
    case "$link_hook" in
    [Yy])
        ln -sf "$SCRIPT_PATH" "$HOOK_PATH"
        ;;
    esac
fi

set -x

# Install tools
cargo clippy --version >/dev/null 2>&1 || rustup component add clippy
cargo machete --version >/dev/null 2>&1 || cargo install --locked cargo-machete
cargo sort --version >/dev/null 2>&1 || cargo install --locked cargo-sort
typos --version >/dev/null 2>&1 || cargo install --locked typos-cli

rustup toolchain list | grep -q 'nightly' || rustup toolchain install nightly
cargo +nightly fmt --version >/dev/null 2>&1 || rustup component add rustfmt --toolchain nightly

# Checks
typos .
cargo machete
cargo +nightly fmt -- --check
cargo sort -c
cargo rustdoc --all-features -- -D warnings
cargo test --all-targets --no-default-features --features json
cargo test --all-targets --no-default-features --features toml
cargo test --all-targets --no-default-features --features yaml
cargo test --all-targets --all-features
cargo test --doc --all-features
cargo clippy --all-features --all-targets -- -D warnings
