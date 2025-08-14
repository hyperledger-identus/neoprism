## Submitter Mode Configuration

The **Submitter node** publishes PRISM DID operations to the Cardano blockchain.  
It is typically used for creating, updating, or deactivating DIDs.

### DLT Sink

The Submitter node currently supports Cardano wallet integration as its DLT sink:

- **Cardano Wallet:**  
  Uses a Cardano wallet to sign and submit transactions containing DID operations.
  - Key options: Wallet base URL, wallet ID, passphrase, payment address.

Configure the wallet integration to match your operational and security requirements.

See [Common Configuration](./common.md) for general server settings.

For a full list of flags and environment variables, see [CLI Options](../references/cli-options.md).
