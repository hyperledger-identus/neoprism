// @ts-types="./managed/contract/index.d.cts"
import * as DidContract from "./managed/contract/index.cjs";
import { ContractState } from "@midnight-ntwrk/ledger";
import { decodeHex } from "@std/encoding/hex";

export function decodeContractState(
  contractStateHex: string,
  networkId: number,
): DidDocument {
  if (contractStateHex.length % 2 !== 0) throw new Error("Hex string must have an even length");
  const buffer = decodeHex(contractStateHex);
  const state = ContractState.deserialize(buffer, networkId);
  const ledger = DidContract.ledger(state.data);
  return "Hello";
}

type DidDocument = string;

