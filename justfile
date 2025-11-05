# PostgreSQL configuration
db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"

# show available commands
default:
    @just --list

# Format all source files (nix, toml, dhall, rust, sql)
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

# Build Tailwind CSS assets
[working-directory: 'bin/neoprism-node']
_build-assets:
    tailwindcss -i tailwind.css -o ./assets/styles.css

# Build dhall configurations
[working-directory: 'docker/.config']
build-config:
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-dbsync" > "../mainnet-dbsync/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-relay" > "../mainnet-relay/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).preprod-relay" > "../preprod-relay/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).prism-test" > "../prism-test/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).prism-test-ci" > "../prism-test/compose-ci.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).mainnet-universal-resolver" > "../mainnet-universal-resolver/compose.yml"
  dhall-to-yaml --generated-comment <<< "(./main.dhall).blockfrost-neoprism-demo" > "../blockfrost-neoprism-demo/compose.yml"

# Build the entire project (assets + cargo)
build: _build-assets
    cargo build --all-features

# Clean workspace
clean:
    cargo clean

# Run neoprism-node with database connection (pass arguments after --)
run *ARGS: _build-assets
    #!/usr/bin/env bash
    export NPRISM_DB_URL="postgres://{{db_user}}:{{db_pass}}@localhost:{{db_port}}/{{db_name}}"
    cargo run --bin neoprism-node -- {{ARGS}}

# Run all tests with all features
test:
    cargo test --all-features

# Start local PostgreSQL database
db-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{db_name}} \
      -e POSTGRES_USER={{db_user}} \
      -e POSTGRES_PASSWORD={{db_pass}} \
      -p {{db_port}}:5432 postgres:16

# Stop local PostgreSQL database
db-down:
    docker stop prism-db

# Dump database to postgres.dump file
db-dump:
    #!/usr/bin/env bash
    export PGPASSWORD={{db_pass}}
    pg_dump -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} -Fc > postgres.dump
    echo "Database dumped to postgres.dump"

# Restore database from postgres.dump file
db-restore:
    #!/usr/bin/env bash
    export PGPASSWORD={{db_pass}}
    pg_restore -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} postgres.dump
    echo "Database restored from postgres.dump"

#------------ prism-test ------------

# Start PRISM test environment
[working-directory: 'docker/prism-test']
prism-test-up:
    docker-compose up -d

# Stop PRISM test environment and remove volumes
[working-directory: 'docker/prism-test']
prism-test-down:
    docker-compose down --volumes

# Run PRISM conformance test
[working-directory: 'tests/prism-test']
prism-test: prism-test-build
    just _prism-test-up
    sbt test
    just _prism-test-down

# Compile PRISM conformance test
[working-directory: 'tests/prism-test']
prism-test-build:
    sbt clean scalafmtAll

