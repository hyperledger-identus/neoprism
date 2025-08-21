// @ts-types="./managed/contract/index.d.cts"
import Api from "./managed/contract/index.cjs";
import { ContractState } from "@midnight-ntwrk/ledger";

export function decodeContractState(
  contractStateHex: string,
  networkId: number,
): string {
  const buffer = hexToUint8Array(contractStateHex);
  const state = ContractState.deserialize(buffer, networkId);
  const contractState = Api.ledger(state.data);
  console.log(contractState);
  return "Hello";
}

function hexToUint8Array(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) throw new Error("Hex string must have an even length");

  return new Uint8Array(
    hex.match(/.{2}/g)!.map(byte => parseInt(byte, 16))
  );
}
