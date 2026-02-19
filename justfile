mod e2e 'tools/just-recipes/e2e.just'
mod tools 'tools/just-recipes/tools.just'
mod release 'tools/just-recipes/release.just'

# PostgreSQL configuration

db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"
postgres_db_url := "postgres://postgres:postgres@localhost:5432/postgres"

# Embedded SQLite defaults

sqlite_db_path := "data/sqlite/neoprism-dev.sqlite"
sqlite_db_url := "sqlite://data/sqlite/neoprism-dev.sqlite"

# Show available commands
default:
    @just --list --list-submodules

# Install npm dependencies
[group('neoprism')]
init:
    npm install

# Build the entire project
[group('neoprism')]
build: build-assets build-config
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

# Run neoprism-node with development database connection
[group('neoprism')]
run *ARGS: build-assets
    export NPRISM_DB_URL="sqlite::memory:" && \
        cargo run --bin neoprism-node -- {{ ARGS }}

# Run all tests with all features enabled
[group('neoprism')]
test:
    cargo test --all-features

# Run tests with code coverage (requires cargo-llvm-cov)
[group('neoprism')]
coverage:
    cargo llvm-cov test --all-features --lcov --output-path lcov.info
    cargo llvm-cov report --all-features
    echo "Coverage report: lcov.info (use 'cargo llvm-cov report --html' for HTML)"

# Generate HTML coverage report
[group('neoprism')]
coverage-html: coverage
    cargo llvm-cov report --all-features --html
    echo "HTML report saved to target/llvm-cov/html/index.html"

# Clean all build artifacts
[group('neoprism')]
clean:
    cargo clean

# Format all source files (Nix, TOML, Rust, Python, SQL)
[group('neoprism')]
format: tools::format
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

# Run fast checks (tests, formatting, lints)
[group('checks')]
check: clean format build test tools::check
    echo "✓ All checks completed successfully."

# Run full checks before submitting a PR, including end-to-end tests
[group('checks')]
full-check: check e2e::run
    #!/usr/bin/env bash
    SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    nix build ".#checks.$SYSTEM.default"
    echo "✓ All full checks completed successfully."

# Start local PostgreSQL database in Docker
[group('database')]
postgres-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{ db_name }} \
      -e POSTGRES_USER={{ db_user }} \
      -e POSTGRES_PASSWORD={{ db_pass }} \
      -p {{ db_port }}:5432 postgres:16

# Stop local PostgreSQL database
[group('database')]
postgres-down:
    docker stop prism-db

# Dump PostgreSQL database to postgres.dump file
[group('database')]
postgres-dump:
    export PGPASSWORD={{ db_pass }} && \
        pg_dump -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} -Fc > postgres.dump && \
        echo "Database dumped to postgres.dump"

# Restore PostgreSQL database from postgres.dump file
[group('database')]
postgres-restore:
    export PGPASSWORD={{ db_pass }} && \
        pg_restore -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} postgres.dump && \
        echo "Database restored from postgres.dump"

# Initialize the embedded SQLite database
[group('database')]
sqlite-init:
    mkdir -p "$(dirname {{ sqlite_db_path }})"
    touch {{ sqlite_db_path }}
    echo "SQLite database initialized at {{ sqlite_db_path }}"

# Remove the embedded SQLite database file
[group('database')]
sqlite-clean:
    rm -f {{ sqlite_db_path }}
    echo "Removed SQLite database at {{ sqlite_db_path }}"
