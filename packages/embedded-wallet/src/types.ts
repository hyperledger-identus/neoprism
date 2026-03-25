export type Network = "mainnet" | "preprod" | "preview" | "custom";

export interface BuildOptions {
  blockfrostUrl?: string;
  blockfrostApiKey?: string;
  mnemonicStdin: boolean;
  prismObjectHex: string;
  network: Network;
}

export const VALID_NETWORKS: readonly Network[] = ["mainnet", "preprod", "preview", "custom"] as const;

export function isValidNetwork(value: string): value is Network {
  return VALID_NETWORKS.includes(value as Network);
}
