# NeoPRISM Installation Guide

This guide will help you install and run a NeoPRISM node using either Docker or Nix.

## Prerequisites

- **Docker**: [Install Docker](https://docs.docker.com/get-docker/)
- **Git**: [Install Git](https://git-scm.com/downloads)

> **Note:** During development and testing, NeoPRISM typically uses between 10–100 MB of memory. No special hardware requirements are expected.

---

## 1. Using Docker

### Quick Start (Mainnet Relay Example)

1. **Clone the NeoPRISM repository:**
   ```bash
   git clone https://github.com/hyperledger-identus/neoprism.git
   cd neoprism/docker/mainnet-relay
   ```

2. **Start NeoPRISM and PostgreSQL using Docker Compose:**
   ```bash
   docker-compose up
   ```

3. **Access the Web UI:**
   - Open [http://localhost:8080](http://localhost:8080) in your browser.

4. **Resolve a DID using the API:**
   ```bash
   curl http://localhost:8080/api/dids/<did>
   ```

---

## 2. Using Nix to build binary (with Flake)

NeoPRISM can be built and run using [Nix flakes](https://nixos.wiki/wiki/Flakes).

### Quick Start

1. **Build the NeoPRISM binary from the remote flake:**
   ```bash
   nix build github:hyperledger-identus/neoprism/<TAG>#neoprism-bin
   ```
   - The resulting binary will be located in `./result/bin/neoprism-node`.

2. **Build the UI assets from the remote flake (in a separate output directory):**
   ```bash
   nix build github:hyperledger-identus/neoprism/<TAG>#neoprism-ui-assets -o ./result-ui-assets
   ```
   - The UI assets will be available in `./result-ui-assets`.

3. **Run NeoPRISM and link the UI assets:**
   - Use the `--assets-path` flag to specify the UI assets directory:
     ```bash
     ./result/bin/neoprism-node indexer --assets-path ./result-ui-assets [options]
     ```
   - For details on available commands and options, see the CLI help:
     ```bash
     ./result/bin/neoprism-node indexer --help
     ```

4. **Access the Web UI:**
   - Open [http://localhost:8080](http://localhost:8080) in your browser.

---

**Note:**  
- You need Nix with flake support enabled. See [Nix Flakes documentation](https://nixos.wiki/wiki/Flakes) for setup instructions.

---

## More Advanced Options

NeoPRISM supports additional deployment modes and Cardano data sources (such as DB Sync and testnet environments). For details on advanced configurations, see the relevant documentation pages.

