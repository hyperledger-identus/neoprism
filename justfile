mod e2e 'tools/just-recipes/e2e.just'
mod tools 'tools/just-recipes/tools.just'
mod release 'tools/just-recipes/release.just'

# PostgreSQL configuration

db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"

# Embedded SQLite defaults
sqlite_db_path := "data/sqlite/neoprism-dev.sqlite"
sqlite_db_url := "sqlite://data/sqlite/neoprism-dev.sqlite"

# Use bash with strict error handling for all recipes
set shell := ["bash", "-euo", "pipefail", "-c"]

# Show available commands
default:
    @just --list --list-submodules

# Install npm dependencies
[group('neoprism')]
init:
    npm install

# Build the entire project (assets + cargo)
[group('neoprism')]
build: build-assets
    cargo build --all-features

# Build Tailwind CSS assets
[group('neoprism')]
[working-directory('bin/neoprism-node')]
build-assets:
    tailwindcss -i tailwind.css -o ./assets/styles.css

# Build Docker Compose configurations from Python sources
[group('neoprism')]
[working-directory('tools')]
build-config:
    python -m compose_gen.main

# Run neoprism-node with local database connection (pass arguments after --)
[group('neoprism')]
run *ARGS: build-assets
    export NPRISM_DB_URL="postgres://{{ db_user }}:{{ db_pass }}@localhost:{{ db_port }}/{{ db_name }}" && \
        cargo run --bin neoprism-node -- {{ ARGS }}

# Run all tests with all features enabled
[group('neoprism')]
test:
    cargo test --all-features

# Run comprehensive Nix checks (format, lint, test, clippy)
[group('neoprism')]
check:
    #!/usr/bin/env bash
    SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    nix build ".#checks.$SYSTEM.default"

# Clean all build artifacts
[group('neoprism')]
clean:
    cargo clean

# Format all source files (Nix, TOML, Rust, Python, SQL)
[group('neoprism')]
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

# Start local PostgreSQL database in Docker (Postgres only)
[group('neoprism')]
db-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{ db_name }} \
      -e POSTGRES_USER={{ db_user }} \
      -e POSTGRES_PASSWORD={{ db_pass }} \
      -p {{ db_port }}:5432 postgres:16

# Stop local PostgreSQL database (Postgres only)
[group('neoprism')]
db-down:
    docker stop prism-db

# Dump local PostgreSQL database to postgres.dump file
[group('neoprism')]
db-dump:
    export PGPASSWORD={{ db_pass }} && \
        pg_dump -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} -Fc > postgres.dump && \
        echo "Database dumped to postgres.dump"

# Restore local PostgreSQL database from postgres.dump file
[group('neoprism')]
db-restore:
    export PGPASSWORD={{ db_pass }} && \
        pg_restore -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} postgres.dump && \
        echo "Database restored from postgres.dump"

# Initialize or upgrade the embedded SQLite database
[group('neoprism')]
db-init-sqlite:
    mkdir -p "$(dirname {{ sqlite_db_path }})"
    touch {{ sqlite_db_path }}
    DATABASE_URL={{ sqlite_db_url }} sqlx migrate run --source lib/node-storage/migrations/sqlite
    echo "SQLite database migrated at {{ sqlite_db_path }}"

# Remove the embedded SQLite database file
[group('neoprism')]
db-clean-sqlite:
    rm -f {{ sqlite_db_path }}
    echo "Removed SQLite database at {{ sqlite_db_path }}"
