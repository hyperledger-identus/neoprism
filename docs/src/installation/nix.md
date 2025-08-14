# Nix Installation

NeoPRISM can be built and run using [Nix flakes](https://nixos.wiki/wiki/Flakes).

## Prerequisites

- **Nix** with flake support enabled. See [Nix Flakes documentation](https://nixos.wiki/wiki/Flakes) for setup instructions.
- **Git**: [Install Git](https://git-scm.com/downloads)

---

## Quick Start

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
