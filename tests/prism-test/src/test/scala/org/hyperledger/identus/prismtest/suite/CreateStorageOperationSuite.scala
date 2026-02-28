package org.hyperledger.identus.prismtest.suite

import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*

object CreateStorageOperationSuite extends StorageTestUtils:

  def allSpecs = suite("CreateStorageOperation")(
    signatureSpec,
    deactivatedSpec,
    nonceSpec
  )

  private def deactivatedSpec = suite("Deactivated DID")(
    test("should reject storage creation by deactivated DID") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .deactivateDid(spo1.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo3.getOperationHash.get)
      yield assert(storage)(isNone)
    },
    test("should remove storage on DID deactivation") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo3 = builder(seed)
          .deactivateDid(spo2.getOperationHash.get, did)
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isNone)
    }
  )

  private def nonceSpec = suite("Nonce")(
    test("should reject duplicate storage with same nonce") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo3 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    },
    test("should accept duplicate storage with different nonce") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did, Array(0))
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo3 = builder(seed)
          .createStorage(did, Array(1))
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage1 <- getVdrEntryHex(spo2.getOperationHash.get)
        storage2 <- getVdrEntryHex(spo3.getOperationHash.get)
      yield assert(storage1)(isSome(equalTo("00"))) && assert(storage2)(isSome(equalTo("00")))
    },
    test("should accept different storage with same nonce") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did, Array(0))
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo3 = builder(seed)
          .createStorage(did, Array(0))
          .bytes("01".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage1 <- getVdrEntryHex(spo2.getOperationHash.get)
        storage2 <- getVdrEntryHex(spo3.getOperationHash.get)
      yield assert(storage1)(isSome(equalTo("00"))) && assert(storage2)(isSome(equalTo("01")))
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should accept signature by VDR key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    },
    test("should reject signature by non-matching VDR key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isNone)
    },
    test("should reject signature by non-VDR key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isNone)
    },
    test("should reject signature by removed VDR key") {
      for
        seed <- newSeed
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        did = spo1.getDid.get
        spo2 = builder(seed)
          .updateDid(spo1.getOperationHash.get, did)
          .removeKey("vdr-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo3 = builder(seed)
          .createStorage(did)
          .bytes("00".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo3.getOperationHash.get)
      yield assert(storage)(isNone)
    }
  )
