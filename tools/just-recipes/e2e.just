set working-directory := '../..'

# Start PRISM conformance test environment
up NAME:
    docker-compose -f docker/prism-test/compose-{{ NAME }}.yml up -d

# Stop PRISM conformance test environment and remove volumes
down NAME:
    docker-compose -f docker/prism-test/compose-{{ NAME }}.yml down --volumes

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile

# Clean PRISM conformance test build artifacts
[working-directory('./tests/prism-test')]
clean:
    sbt clean

# Run all PRISM conformance test suites
[working-directory('./tests/prism-test')]
run-all: docker-publish-local
    just run dev

# Run PRISM conformance tests against specific configuration
[working-directory('./tests/prism-test')]
run NAME:
    just e2e::up {{ NAME }}
    sbt test
    just e2e::down {{ NAME }}

# Build and load Docker images for local testing
[working-directory('./tests/prism-test')]
docker-publish-local:
    nix build .#neoprism-docker-latest && docker load < ./result
    nix build .#cardano-testnet-docker && docker load < ./result
