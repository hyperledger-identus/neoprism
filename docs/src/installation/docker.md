# Docker Installation

This guide will help you install and run a NeoPRISM node using Docker.

## Prerequisites

- **Docker**: [Install Docker](https://docs.docker.com/get-docker/)
- **Git**: [Install Git](https://git-scm.com/downloads)

## Steps (Mainnet Relay Example)

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

> **Note:** The example above demonstrates one way to run NeoPRISM using Docker. You can find additional deployment examples and configurations in the `docker` directory of the repository.  
>  
> If you are deploying NeoPRISM in a production environment, please take extra care to harden your setup according to your organization's security requirements.
