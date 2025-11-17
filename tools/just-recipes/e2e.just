set working-directory := '../..'

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile

# Clean PRISM conformance test build artifacts
[working-directory('./tests/prism-test')]
clean:
    sbt clean

# Start PRISM conformance test environment
up NAME:
    docker-compose -f docker/prism-test/compose-{{ NAME }}.yml up -d --wait

# Stop PRISM conformance test environment and remove volumes
down NAME:
    docker-compose -f docker/prism-test/compose-{{ NAME }}.yml down --volumes

# Run all PRISM conformance test suites
[working-directory('./tests/prism-test')]
run: docker-publish-local
    cowsay "Testing dev configuration"
    just e2e::up dev
    SKIP_CONFIRMATION_CHECK_MILLIS=2000 sbt test
    just e2e::down dev

    cowsay "Testing ci configuration"
    just e2e::up ci
    sbt test
    just e2e::down ci

# Build and load Docker images for local testing
[working-directory('./tests/prism-test')]
docker-publish-local:
    nix build .#neoprism-docker-latest && docker load < ./result
