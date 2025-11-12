# PostgreSQL configuration
db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"

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

# Build Docker Compose configurations from Python sources
[group: 'neoprism']
build-config:
  python -m tools.compose_gen.main

# Run neoprism-node with local database connection (pass arguments after --)
[group: 'neoprism']
run *ARGS: build-assets
    export NPRISM_DB_URL="postgres://{{db_user}}:{{db_pass}}@localhost:{{db_port}}/{{db_name}}" && \
        cargo run --bin neoprism-node -- {{ARGS}}

# Run all tests with all features enabled
[group: 'neoprism']
test:
    cargo test --all-features

# Clean all build artifacts
[group: 'neoprism']
clean:
    cargo clean

# Format all source files (Nix, TOML, Rust, Python, SQL)
[group: 'neoprism']
format:
    echo "Formatting Nix files..."
    find . -name '*.nix' -type f -exec sh -c 'echo "  → {}" && nixfmt {}' \;
    
    echo "Formatting TOML files..."
    find . -name '*.toml' -type f -exec sh -c 'echo "  → {}" && taplo format {}' \;
    
    echo "Formatting Python files..."
    find tools/compose_gen -name '*.py' -type f -exec sh -c 'echo "  → {}" && ruff check --select I --fix {} && ruff format {}' \;
    
    echo "Formatting Rust files..."
    cargo fmt
    
    echo "Formatting Hurl files..."
    find . -name '*.hurl' -type f -exec sh -c 'echo "  → {}" && hurlfmt --in-place {}' \;
    
    echo "Formatting SQL files..."
    cd lib/node-storage/migrations && \
        sqlfluff fix . && \
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
    export PGPASSWORD={{db_pass}} && \
        pg_dump -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} -Fc > postgres.dump && \
        echo "Database dumped to postgres.dump"

# Restore local database from postgres.dump file
[group: 'neoprism']
db-restore:
    export PGPASSWORD={{db_pass}} && \
        pg_restore -h localhost -p {{db_port}} -U {{db_user}} -w -d {{db_name}} postgres.dump && \
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
    sbt test

# Build PRISM conformance test suite
[group: 'prism-test']
[working-directory: 'tests/prism-test']
prism-test-build:
    sbt clean scalafmtAll

# Automatically bump version using git-cliff
[group: 'release']
release-bump:
    #!/usr/bin/env bash
    set -euxo pipefail
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
    #!/usr/bin/env bash
    set -euxo pipefail
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

