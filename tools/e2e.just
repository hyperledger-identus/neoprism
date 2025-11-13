# Start PRISM conformance test environment
[working-directory('../docker/prism-test')]
up:
    docker-compose up -d

# Stop PRISM conformance test environment and remove volumes
[working-directory('../docker/prism-test')]
down:
    docker-compose down --volumes

# Run PRISM conformance tests
[working-directory('../tests/prism-test')]
run: build
    sbt test

# Build PRISM conformance test suite
[working-directory('../tests/prism-test')]
build:
    sbt clean scalafmtAll compile Test/compile
