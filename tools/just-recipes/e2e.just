set working-directory := '../..'

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt scalafmtAll compile Test/compile

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

# Run all PRISM conformance test suites with isolated cleanup and receipts
run:
    python tools/e2e_runner.py

# Run the offline E2E runner tests
test-runner:
    PYTHONPATH=tools python -m unittest discover -s tools/tests -p 'test_e2e_runner.py'

# Build and load Docker images for local testing
docker-publish-local:
    nix build .#neoprism-docker-latest && docker load < ./result
