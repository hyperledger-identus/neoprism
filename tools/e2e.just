set working-directory := '..'

# Start PRISM conformance test environment
up:
    docker-compose -f docker/prism-test/docker-compose.yml up -d

# Stop PRISM conformance test environment and remove volumes
down:
    docker-compose -f docker/prism-test/docker-compose.yml down --volumes

# Run PRISM conformance tests
[working-directory('./tests/prism-test')]
run: build
    sbt test

# Build PRISM conformance test suite
[working-directory('./tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile
