// @ts-types="./managed/contract/index.d.cts"
import * as DidContract from "./managed/contract/index.cjs";
import { ContractState } from "@midnight-ntwrk/ledger";
import { decodeHex } from "@std/encoding/hex";
import { DidDocument } from "../../../lib/did-core/bindings/did_core_types.ts";

export function decodeContractState(
  networkId: number,
  contractStateHex: string,
): DidDocument {
  const buffer = decodeHex(contractStateHex);
  const state = ContractState.deserialize(buffer, networkId);
  const ledger: DidContract.Ledger = DidContract.ledger(state.data);
  const didDocument: DidDocument = {
    context: [],
    id: "",
    verificationMethod: [],
    authentication: [],
    assertionMethod: [],
    keyAgreement: [],
    capabilityInvocation: [],
    capabilityDelegation: [],
    service: [],
  };
  return didDocument;
}
