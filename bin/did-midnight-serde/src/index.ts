import DidContract from "./managed/did/contract/index.cjs";
import { ContractState } from "@midnight-ntwrk/ledger";
// Node.js Buffer is used for hex decoding
function decodeHex(hex: string): Buffer {
  return Buffer.from(hex, 'hex');
}
import {
  DidDocument,
  Service,
  VerificationMethod,
} from "../../../lib/did-core/bindings/did_core_types";

export function decodeContractState(
  networkId: number,
  contractStateHex: string,
): DidDocument {
  const buffer = decodeHex(contractStateHex);
  const state = ContractState.deserialize(buffer, networkId);
  const ledger = DidContract.ledger(state.data);
  const didDocument: DidDocument = {
    context: [], // W3C context, left empty for now
    id: "did:example:todo", // Hardcoded for now
    verificationMethod: mapVerificationMethods(ledger),
    authentication: mapRelation(ledger.authenticationRelation),
    assertionMethod: mapRelation(ledger.assertionMethodRelation),
    keyAgreement: mapRelation(ledger.keyAgreementRelation),
    capabilityInvocation: mapRelation(ledger.capabilityInvocationRelation),
    capabilityDelegation: mapRelation(ledger.capabilityDelegationRelation),
    service: mapServices(ledger),
  };
  return didDocument;
}

function mapVerificationMethods(
  ledger: DidContract.Ledger,
): VerificationMethod[] {
  const methods: VerificationMethod[] = [];
  for (const [, method] of ledger.verificationMethods) {
    methods.push({
      id: "TODO",
      type: DidContract.VerificationMethodType[method.type],
      controller: "TODO",
      publicKeyJwk: {
        x: "TODO",
        y: "TODO",
        kty: "TODO",
        crv: "TODO",
      },
    });
  }
  return methods;
}

function mapRelation(
  relation: { [Symbol.iterator](): Iterator<string> },
): string[] {
  const arr: string[] = [];
  for (const id of relation) {
    arr.push(id);
  }
  return arr;
}

function mapServices(ledger: DidContract.Ledger): Service[] {
  const services: Service[] = [];
  for (const [id, svc] of ledger.services) {
    services.push({
      id,
      type: svc.type,
      serviceEndpoint: Array.isArray(svc.serviceEndpoint)
        ? svc.serviceEndpoint
        : [svc.serviceEndpoint],
    });
  }
  return services;
}
