import DidContract from "./managed/did/contract/index.cjs";
import { ContractState } from "@midnight-ntwrk/ledger";
import {
  Did,
  DidDocument,
  Service,
  VerificationMethod,
} from "../../../lib/did-core/bindings/did_core_types";

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
        x: method.publicKeyJwk.x,
        y: method.publicKeyJwk.y,
        kty: method.publicKeyJwk.kty,
        crv: method.publicKeyJwk.crv,
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
