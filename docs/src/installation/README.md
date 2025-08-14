# NeoPRISM Installation Guide

This guide will help you install and run a NeoPRISM node using Docker.

## Prerequisites

- **Docker**: [Install Docker](https://docs.docker.com/get-docker/)
- **Git**: [Install Git](https://git-scm.com/downloads)

> **Note:** During development and testing, NeoPRISM typically uses between 10–100 MB of memory. No special hardware requirements are expected.

## Quick Start (Mainnet Relay Example)

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

## More Advanced Options

This guide covers the simplest mainnet-relay setup. NeoPRISM supports additional deployment modes and Cardano data sources (such as DB Sync and testnet environments). For details on advanced configurations, see the relevant documentation pages.

