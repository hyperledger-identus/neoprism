set working-directory := '../..'

# Start PRISM conformance test environment
up ARG:
    docker-compose -f docker/prism-test/compose-{{ ARG }}.yml up -d

# Stop PRISM conformance test environment and remove volumes
down ARG:
    docker-compose -f docker/prism-test/compose-{{ ARG }}.yml down --volumes

# Run PRISM conformance tests
[working-directory('./tests/prism-test')]
run:
    nix build .#neoprism-docker-latest && docker load < ./result
    nix build .#cardano-testnet-docker && docker load < ./result
    just e2e::up ci
    just e2e::down ci

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile

# Clean PRISM conformance test build artifacts
[working-directory('./tests/prism-test')]
clean:
    sbt clean
