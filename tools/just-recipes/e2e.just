set working-directory := '../..'

# Start PRISM conformance test environment
up:
    docker-compose -f docker/prism-test/compose.yml up -d

# Stop PRISM conformance test environment and remove volumes
down:
    docker-compose -f docker/prism-test/compose.yml down --volumes

# Run PRISM conformance tests
[working-directory('./tests/prism-test')]
run:
    sbt test

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile

# Clean PRISM conformance test build artifacts
[working-directory('./tests/prism-test')]
clean:
    sbt clean
