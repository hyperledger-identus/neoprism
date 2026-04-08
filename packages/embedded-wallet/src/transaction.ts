import { MeshTxBuilder, BlockfrostProvider } from "@meshsdk/core";
import { MeshWallet } from "@meshsdk/wallet";
import type { Network } from "./types";
import { normalizeHex, hexToBytes } from "./hex";

export interface BuildTransactionParams {
  mnemonic: string[];
  network: Network;
  blockfrostUrl?: string;
  blockfrostApiKey?: string;
  prismObjectHex: string;
}

export interface BuiltTransaction {
  cbor: string;
}

const NETWORK_IDS: Record<Network, 0 | 1> = {
  mainnet: 1,
  preprod: 0,
  preview: 0,
  custom: 0,  // Custom testnets always use testnet addresses (network_id=0)
};

/**
 * Encodes a hex string to the Cardano metadata format for label 21325.
 * The PrismObject bytes are split into 64-byte chunks as per PRISM specification.
 * Returns a Map suitable for passing to MeshSDK's metadataValue function.
 */
function encodePrismObjectToMetadata(prismObjectHex: string): Map<string, unknown> {
  const hex = normalizeHex(prismObjectHex);

  // Split into 64-byte (128 hex character) chunks and convert to Uint8Array
  const chunks: Uint8Array[] = [];
  for (let i = 0; i < hex.length; i += 128) {
    const chunkHex = hex.slice(i, i + 128);
    chunks.push(hexToBytes(chunkHex));
  }

  // Build metadata structure with v/c format using Map for proper Cardano encoding
  const metadataMap = new Map<string, unknown>();
  metadataMap.set("v", 1);
  metadataMap.set("c", chunks);

  return metadataMap;
}

export async function buildTransaction(params: BuildTransactionParams): Promise<BuiltTransaction> {
  const { mnemonic, network, blockfrostUrl, blockfrostApiKey, prismObjectHex } = params;

  const cardanoMetadata = encodePrismObjectToMetadata(prismObjectHex);

  // BlockfrostProvider accepts either an API key (for public Blockfrost) or a URL
  // (for self-hosted/private instances). We use ?? to prefer the API key if provided,
  // falling back to the URL for private deployments where no API key is needed.
  const provider = new BlockfrostProvider(blockfrostApiKey ?? blockfrostUrl!);

  const networkId: 0 | 1 = NETWORK_IDS[network];

  const wallet = new MeshWallet({
    networkId,
    fetcher: provider,
    submitter: provider,
    key: {
      type: "mnemonic",
      words: mnemonic,
    },
    accountType: "payment",
  });

  await wallet.init();

  const address = await wallet.getChangeAddress();

  const utxos = await wallet.getUtxos();

  if (utxos.length === 0) {
    throw new Error(`no UTXOs found at address ${address}`);
  }

  const txBuilder = new MeshTxBuilder({
    fetcher: provider,
    submitter: provider,
  });

  txBuilder.selectUtxosFrom(utxos);
  txBuilder.changeAddress(address);
  txBuilder.metadataValue(21325, cardanoMetadata);

  const unsignedTx = await txBuilder.complete();

  const signedTx = await wallet.signTx(unsignedTx, false, true);

  return {
    cbor: signedTx,
  };
}
