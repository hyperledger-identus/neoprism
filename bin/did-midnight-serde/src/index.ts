import DidContract from "./managed/did/contract/index.cjs";
import {
  Did,
  DidDocument,
  Service,
  VerificationMethod,
} from "../../../lib/did-core/bindings/did_core_types";
import { ContractState } from "@midnight-ntwrk/compact-runtime";

export function decodeContractState(
  did: Did,
  networkId: number,
  contractStateHex: string,
): DidDocument {
  const buffer = Buffer.from(contractStateHex, 'hex');
  const state = ContractState.deserialize(buffer, networkId);
  const ledger = DidContract.ledger(state.data);
  const didDocument: DidDocument = {
    context: [],
    id: did,
    verificationMethod: mapVerificationMethods(did, ledger),
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
  did: Did,
  ledger: DidContract.Ledger,
): VerificationMethod[] {
  const methods: VerificationMethod[] = [];
  for (const [id, method] of ledger.verificationMethods) {
    methods.push({
      id,
      type: DidContract.VerificationMethodType[method.type],
      controller: did,
      publicKeyJwk: {
        x: bigintToBase64Url(method.publicKeyJwk.x),
        y: bigintToBase64Url(method.publicKeyJwk.y),
        kty: DidContract.KeyType[method.publicKeyJwk.kty],
        crv: DidContract.CurveType[method.publicKeyJwk.crv],
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

function bigintToBase64Url(n: bigint): string {
  let hex = n.toString(16);
  if (hex.length % 2) hex = "0" + hex;
  return Buffer.from(hex, "hex").toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
