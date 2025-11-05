# PostgreSQL configuration
db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"

# Show available commands
default:
    @just --list

# Build the entire project (assets + cargo)
[group: 'neoprism']
build: build-assets
    cargo build --all-features

# Build Tailwind CSS assets
[group: 'neoprism']
[working-directory: 'bin/neoprism-node']
build-assets:
    tailwindcss -i tailwind.css -o ./assets/styles.css

# Build Docker Compose configurations from Dhall sources
[group: 'neoprism']
[working-directory: 'docker/.config']
build-config:
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-dbsync" > "../mainnet-dbsync/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-relay" > "../mainnet-relay/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).preprod-relay" > "../preprod-relay/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).prism-test" > "../prism-test/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).prism-test-ci" > "../prism-test/compose-ci.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-universal-resolver" > "../mainnet-universal-resolver/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).blockfrost-neoprism-demo" > "../blockfrost-neoprism-demo/compose.yml"

# Run neoprism-node with local database connection (pass arguments after --)
[group: 'neoprism']
run *ARGS: build-assets
    #!/usr/bin/env bash
    export NPRISM_DB_URL="postgres://{{db_user}}:{{db_pass}}@localhost:{{db_port}}/{{db_name}}"
    cargo run --bin neoprism-node -- {{ARGS}}

# Run all tests with all features enabled
[group: 'neoprism']
test:
    cargo test --all-features

# Clean all build artifacts
[group: 'neoprism']
clean:
    cargo clean

# Format all source files (Nix, TOML, Dhall, Rust, SQL)
[group: 'neoprism']
format:
    set -euo pipefail
    
    echo "Formatting Nix files..."
    find . -name '*.nix' -type f -exec sh -c 'echo "  → {}" && nixfmt {}' \;
    
    echo "Formatting TOML files..."
    find . -name '*.toml' -type f -exec sh -c 'echo "  → {}" && taplo format {}' \;
    
    echo "Formatting Dhall files..."
    find . -name '*.dhall' -type f -exec sh -c 'echo "  → {}" && dhall format {}' \;
    
    echo "Formatting Rust files..."
    cargo fmt
    
    echo "Formatting SQL files..."
    cd lib/node-storage/migrations
    sqlfluff fix .
    sqlfluff lint .

# Start local PostgreSQL database in Docker
[group: 'neoprism']
db-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{db_name}} \
      -e POSTGRES_USER={{db_user}} \
      -e POSTGRES_PASSWORD={{db_pass}} \
      -p {{db_port}}:5432 postgres:16

# Stop local PostgreSQL database
[group: 'neoprism']
db-down:
    docker stop prism-db

# Dump local database to postgres.dump file
[group: 'neoprism']
db-dump:
    #!/usr/bin/env bash
    export PGPASSWORD={{db_pass}}
    pg_dump -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} -Fc > postgres.dump
    echo "Database dumped to postgres.dump"

# Restore local database from postgres.dump file
[group: 'neoprism']
db-restore:
    #!/usr/bin/env bash
    export PGPASSWORD={{db_pass}}
    pg_restore -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} postgres.dump
    echo "Database restored from postgres.dump"

# Start PRISM conformance test environment
[group: 'prism-test']
[working-directory: 'docker/prism-test']
prism-test-up:
    docker-compose up -d

# Stop PRISM conformance test environment and remove volumes
[group: 'prism-test']
[working-directory: 'docker/prism-test']
prism-test-down:
    docker-compose down --volumes

# Run PRISM conformance tests
[group: 'prism-test']
[working-directory: 'tests/prism-test']
prism-test-run: prism-test-build
    just _prism-test-up
    sbt test
    just _prism-test-down

# Build PRISM conformance test suite
[group: 'prism-test']
[working-directory: 'tests/prism-test']
prism-test-build:
    sbt clean scalafmtAll

