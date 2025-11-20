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
    docker-compose -f docker/prism-test/compose-{{ NAME }}.yml down --volumes --remove-orphans

# Run all PRISM conformance test suites
run: docker-publish-local
    for cfg in dev dev-sqlite; do \
        cowsay "Testing ${cfg} configuration"; \
        just e2e::up ${cfg}; \
        (cd tests/prism-test && SKIP_CONFIRMATION_CHECK_MILLIS=2000 sbt test); \
        just e2e::down ${cfg}; \
    done

    for cfg in ci ci-sqlite; do \
        cowsay "Testing ${cfg} configuration"; \
        just e2e::up ${cfg}; \
        (cd tests/prism-test && sbt test); \
        just e2e::down ${cfg}; \
    done

# Build and load Docker images for local testing
docker-publish-local:
    nix build .#neoprism-docker-latest && docker load < ./result
