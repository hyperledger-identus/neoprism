## Submitter Configuration

The **Submitter node** publishes PRISM DID operations to the Cardano blockchain.  
It is typically used for creating, updating, or deactivating DIDs.

### DLT Sink

The Submitter node currently supports Cardano wallet integration as its DLT sink:

- **Cardano Wallet:**  
  Uses a Cardano wallet to sign and submit transactions containing DID operations.
  - Key options: 
    - Wallet base URL: `--wallet-base-url` or `NPRISM_WALLET_BASE_URL`
    - Wallet ID: `--wallet-id` or `NPRISM_WALLET_ID`
    - Passphrase: `--wallet-passphrase` or `NPRISM_WALLET_PASSPHRASE`
    - Payment address: `--payment-address` or `NPRISM_PAYMENT_ADDRESS`

> **Important:**
When the submitter publishes a DID operation, it creates a transaction from the configured wallet to the specified payment address.  
Make sure you use your own payment address. Using an incorrect or third-party address may result in permanent loss of funds.

Configure the wallet integration to match your operational and security requirements.

---

**Next Steps:**
- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
