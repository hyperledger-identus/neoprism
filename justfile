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
    @just --list

# Install npm dependencies
[group: 'neoprism']
init:
    npm install

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
    echo "Formatting Nix files..."
    find . -name '*.nix' -type f -exec sh -c 'echo "  → {}" && nixfmt {}' \;
    
    echo "Formatting TOML files..."
    find . -name '*.toml' -type f -exec sh -c 'echo "  → {}" && taplo format {}' \;
    
    echo "Formatting Dhall files..."
    find . -name '*.dhall' -type f -exec sh -c 'echo "  → {}" && dhall format {}' \;
    
    echo "Formatting Rust files..."
    cargo fmt
    
    echo "Formatting SQL files..."
    (cd lib/node-storage/migrations/postgres && sqlfluff fix . && sqlfluff lint .)

# Start local PostgreSQL database in Docker (Postgres only)
[group: 'neoprism']
db-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{db_name}} \
      -e POSTGRES_USER={{db_user}} \
      -e POSTGRES_PASSWORD={{db_pass}} \
      -p {{db_port}}:5432 postgres:16

# Stop local PostgreSQL database (Postgres only)
[group: 'neoprism']
db-down:
    docker stop prism-db

# Dump local PostgreSQL database to postgres.dump file
[group: 'neoprism']
db-dump:
    export PGPASSWORD={{db_pass}}
    pg_dump -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} -Fc > postgres.dump
    echo "Database dumped to postgres.dump"

# Restore local PostgreSQL database from postgres.dump file
[group: 'neoprism']
db-restore:
    export PGPASSWORD={{db_pass}}
    pg_restore -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} postgres.dump
    echo "Database restored from postgres.dump"

# Initialize or upgrade the embedded SQLite database
[group: 'neoprism']
db-init-sqlite:
    mkdir -p "$(dirname {{sqlite_db_path}})"
    touch {{sqlite_db_path}}
    DATABASE_URL={{sqlite_db_url}} sqlx migrate run --source lib/node-storage/migrations/sqlite
    echo "SQLite database migrated at {{sqlite_db_path}}"

# Remove the embedded SQLite database file
[group: 'neoprism']
db-clean-sqlite:
    rm -f {{sqlite_db_path}}
    echo "Removed SQLite database at {{sqlite_db_path}}"

# Export a database snapshot (backend arg: postgres|sqlite)
[group: 'neoprism']
db-backup backend output:
    if [ "{{backend}}" = "postgres" ]; then \
        DB_URL="postgres://{{db_user}}:{{db_pass}}@localhost:{{db_port}}/{{db_name}}"; \
    else \
        DB_URL="{{sqlite_db_url}}"; \
    fi; \
    cargo run --bin neoprism-node -- db backup --db-backend {{backend}} --db-url "$DB_URL" --output "{{output}}"

# Restore a database snapshot (backend arg: postgres|sqlite)
[group: 'neoprism']
db-restore-snapshot backend input:
    if [ "{{backend}}" = "postgres" ]; then \
        DB_URL="postgres://{{db_user}}:{{db_pass}}@localhost:{{db_port}}/{{db_name}}"; \
    else \
        DB_URL="{{sqlite_db_url}}"; \
    fi; \
    cargo run --bin neoprism-node -- db restore --db-backend {{backend}} --db-url "$DB_URL" --input "{{input}}"

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

# Automatically bump version using git-cliff
[group: 'release']
release-bump:
    NEW_VERSION=$(git-cliff --bump --context | jq -r .[0].version | sed s/^v//)
    just release-set "$NEW_VERSION"

# Set project version manually
[group: 'release']
release-set VERSION:
    echo "Setting new version to {{VERSION}}"
    echo "{{VERSION}}" > version
    cargo set-version "{{VERSION}}"
    just build-config
    git-cliff -t "{{VERSION}}" > CHANGELOG.md

# Build and release multi-arch cardano-testnet Docker image
[group: 'release']
release-testnet:
    TAG=$(date +"%Y%m%d-%H%M%S")
    
    echo "Building amd64 image..."
    nix build .#cardano-testnet-docker-linux-amd64 -o result-amd64
    
    echo "Building arm64 image..."
    nix build .#cardano-testnet-docker-linux-arm64 -o result-arm64
    
    echo "Loading images into Docker..."
    docker load < ./result-amd64
    docker load < ./result-arm64
    
    echo "Tagging images with $TAG..."
    docker tag cardano-testnet:latest-amd64 "patextreme/cardano-testnet:$TAG-amd64"
    docker tag cardano-testnet:latest-arm64 "patextreme/cardano-testnet:$TAG-arm64"
    
    rm -rf ./result-amd64 ./result-arm64
    
    echo "Pushing architecture-specific images..."
    docker push "patextreme/cardano-testnet:$TAG-amd64"
    docker push "patextreme/cardano-testnet:$TAG-arm64"
    
    echo "Creating and pushing multi-arch manifest..."
    docker manifest create "patextreme/cardano-testnet:$TAG" \
      "patextreme/cardano-testnet:$TAG-amd64" \
      "patextreme/cardano-testnet:$TAG-arm64"
    docker manifest push "patextreme/cardano-testnet:$TAG"
    
    echo "✓ Released: patextreme/cardano-testnet:$TAG"
