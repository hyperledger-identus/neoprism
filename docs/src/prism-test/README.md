# PRISM Specification Tests

The `prism-test` suite provides conformance tests for PRISM node implementations. These tests help developers verify that their NeoPRISM or PRISM node changes adhere to the PRISM specification.

## Overview

The tests are located in the `tests/prism-test` directory. They cover key PRISM features such as DID creation, update, deactivation, and storage operations, ensuring your node implementation behaves as expected.

## Running the Tests

### 1. Start Required Services

Navigate to the `docker/prism-test` directory and start the required services using Docker Compose:

```sh
cd docker/prism-test
docker-compose up
```

This will launch all necessary dependencies for the test suite.

### 2. Run the Test Suite

In a separate terminal, navigate to the `tests/prism-test` directory and run the tests using `sbt`:

```sh
cd tests/prism-test
sbt test
```

The test results will indicate whether your PRISM node implementation conforms to the specification.

## Who Should Use These Tests?

- **NeoPRISM Developers:** Use these tests when making changes to NeoPRISM to ensure continued compliance.
- **PRISM Node Developers:** Run the suite to validate your node implementation against the PRISM specification.

## Adding a New PRISM Node Implementation

To include a new PRISM node implementation in the test suite:

1. **Edit `MainSpec.scala`:**  
   Go to `tests/prism-test/src/test/scala/org/hyperledger/identus/prismtest/MainSpec.scala` and add your new node to the test suite by providing the appropriate layer and configuration.

2. **Implement NodeClient Interface (if needed):**  
   If your node uses a different RPC or API, you may need to implement the `NodeClient` interface in `tests/prism-test/src/main/scala/org/hyperledger/identus/prismtest/NodeClient.scala` to adapt the test suite to your node's communication protocol.

This allows you to run the conformance tests against your custom PRISM node implementation and verify its compliance with the PRISM specification.
