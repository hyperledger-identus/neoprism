package org.hyperledger.identus.prismtest.suite

import org.hyperledger.identus.prismtest.utils.TestUtils
import org.hyperledger.identus.prismtest.NodeName
import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*

object UpdateDidOperationSuite extends TestUtils:
  // TODO: check if scala-did is patched correctly
  // TODO: add tests for add / remove / update service action
  def allSpecs = suite("UpdateDidOperation")(
    signatureSpec,
    prevOperationHashSpec,
    addPublicKeySpec,
    removePublicKeySpec,
    contextSpec
  ) @@ NodeName.skipIf("scala-did")

  private def contextSpec = suite("Context")(
    test("should accept new context values") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .context("https://www.w3.org/ns/did/v1")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .patchContext(Seq("https://www.w3.org/ns/credentials/v1"))
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.context)(
        hasSameElements(Seq("https://www.w3.org/ns/credentials/v1"))
      )
    },
    test("should reject duplicate context values") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .patchContext(Seq("https://www.w3.org/ns/did/v1", "https://www.w3.org/ns/did/v1"))
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2), batch = false)
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.context)(isEmpty)
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
          .updateDid(Array.fill[Byte](32)(0), did)
          .addKey("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
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
          .updateDid(spo1.getOperationHash.get, did)
          .addKey("master-2")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/2'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0", "master-1")))
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should reject non-existing signing key") {
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
          .signWith("master-2", deriveSecp256k1(seed)("m/0'/1'/2'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should accept signing key being removed") {
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
          .removeKey("master-1")
          .build
          .signWith("master-1", deriveSecp256k1(seed)("m/0'/1'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should reject revoked signing key") {
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
          .removeKey("master-1")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .updateDid(spo2.getOperationHash.get, did)
          .addKey("master-2")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/2'")
          .build
          .signWith("master-1", deriveSecp256k1(seed)("m/0'/1'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    }
  )

  private def addPublicKeySpec = suite("AddPublicKey action")(
    test("should accept new public key") {
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
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0", "master-1")))
    },
    test("should reject duplicate key ID") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .addKey("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/1'")
          .addKey("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/2'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should accept 50 public keys") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = (1 until 50)
          .foldLeft(builder(seed).updateDid(spo1.getOperationHash.get, did)) { case (acc, n) =>
            acc.addKey(s"master-$n")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/$n'")
          }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(50)))
    },
    test("should reject 51 public keys") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = (1 until 51)
          .foldLeft(builder(seed).updateDid(spo1.getOperationHash.get, did)) { case (acc, n) =>
            acc.addKey(s"master-$n")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/$n'")
          }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should accept maximum key ID length (50 chars)") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .addKey("0" * 50)(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(2)))
    },
    test("should reject excessive key ID length (51 chars)") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .addKey("0" * 51)(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2), batch = false)
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should reject re-added key ID") {
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
          .removeKey("master-1")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .updateDid(spo2.getOperationHash.get, did)
          .addKey("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/1'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    }
  )

  private def removePublicKeySpec = suite("RemovePublicKey action")(
    test("should accept existing public key removal") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("master-1")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .removeKey("master-1")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should reject last master key removal") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .removeKey("master-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should reject non-existent key ID removal") {
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
          .removeKey("master-1")
          .removeKey("master-2")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0", "master-1")))
    },
    test("should reject re-removed key") {
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
          .removeKey("master-1")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .updateDid(spo2.getOperationHash.get, did)
          .removeKey("master-1")
          .addKey("master-2")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/2'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        didData <- getDidDocument(did).map(_.get)
      yield assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    }
  )
