mod e2e 'tools/just-recipes/e2e.just'
mod env 'tools/just-recipes/env.just'
mod tools 'tools/just-recipes/tools.just'
mod release 'tools/just-recipes/release.just'

# Use bash with strict error handling for all recipes
set shell := ["bash", "-euo", "pipefail", "-c"]

# Show available commands
default:
    @just --list --list-submodules

# Install npm dependencies
[group('development')]
init:
    npm install

# Build the entire project
[group('development')]
build: build-assets build-config
    cargo build --all-features

# Build Tailwind CSS assets
[group('development')]
[working-directory('bin/neoprism-node')]
build-assets:
    tailwindcss -i tailwind.css -o ./assets/styles.css

# Build Docker Compose configurations from Python sources
[group('development')]
[working-directory('tools')]
build-config:
    python -m compose_gen.main

# Run neoprism-node with local database connection (pass arguments after --)
[group('development')]
run *ARGS: build-assets
    export NPRISM_DB_URL="sqlite::memory:" && \
        cargo run --bin neoprism-node -- {{ ARGS }}

# Run all tests with all features enabled
[group('development')]
test:
    cargo test --all-features

# Clean all build artifacts
[group('development')]
clean:
    cargo clean

# Format all source files (Nix, TOML, Rust, Python, SQL)
[group('development')]
format:
    echo "Formatting Nix files..."
    find . -name '*.nix' -type f -exec sh -c 'echo "  → {}" && nixfmt {}' \;

    echo "Formatting TOML files..."
    find . -name '*.toml' -type f -exec sh -c 'echo "  → {}" && taplo format {}' \;

    echo "Formatting Rust files..."
    cargo fmt

    echo "Formatting Hurl files..."
    find . -name '*.hurl' -type f -exec sh -c 'echo "  → {}" && hurlfmt --in-place {}' \;

    echo "Formatting SQL files..."
    (cd lib/node-storage/migrations/postgres && sqlfluff fix . && sqlfluff lint .)

# Run comprehensive Nix checks (format, lint, test, clippy)
[group('checks')]
check:
    #!/usr/bin/env bash
    SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    nix build ".#checks.$SYSTEM.default"
