package org.hyperledger.identus.prismtest.suite

import io.iohk.atala.prism.protos.node_api.DIDData
import org.hyperledger.identus.prismtest.utils.TestUtils
import org.hyperledger.identus.prismtest.NodeName
import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*

object DeactivateDidOperationSuite extends TestUtils:

  private def assertDidDeactivated(didData: DIDData) =
    assert(didData.publicKeys)(isEmpty) && assert(didData.services)(isEmpty)

  def allSpecs = suite("DeactivateDidOperation")(
    signatureSpec,
    prevOperationHashSpec,
    deactivatedSpec
  ) @@ NodeName.skipIf("scala-did")

  private def deactivatedSpec = suite("Deactivated DID")(
    test("should reject re-creation") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .deactivateDid(spo1.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo1))
        didData <- getDidDocument(did).map(_.get)
      yield assertDidDeactivated(didData)
    }
  )

  private def prevOperationHashSpec = suite("PreviousOperationHash")(
    test("should reject invalid operation hash") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .deactivateDid(Array.fill(32)(0), did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(1)))
    },
    test("should reject non-latest operation hash") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .addKey("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .deactivateDid(spo1.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(2)))
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should accept active master-key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .deactivateDid(spo1.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assertDidDeactivated(didData)
    },
    test("should reject non-existing master-key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .deactivateDid(spo1.getOperationHash.get, did)
          .signWith("master-1", deriveSecp256k1(seed)("m/0'/1'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(1)))
    },
    test("should reject revoked master-key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .removeKey("master-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .deactivateDid(spo2.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(1)))
    }
  )
