package org.hyperledger.identus.prismtest.suite

import proto.prism.SignedPrismOperation
import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*

object UpdateStorageOperationSuite extends StorageTestUtils:

  // TODO: add unknown fields spec
  def allSpecs = suite("UpdateStorageOperation")(
    signatureSpec,
    prevOperationHashSpec
  )

  private def prevOperationHashSpec = suite("PreviousOperationHash")(
    test("should accept update storage with valid operation hash") {
      for
        seed <- newSeed
        updateStorage = (spo: SignedPrismOperation, dataHex: String) =>
          builder(seed)
            .updateStorage(spo.getOperationHash.get)
            .bytes(dataHex.decodeHex)
            .build
            .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
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
        spo3 = updateStorage(spo2, "01")
        spo4 = updateStorage(spo3, "02")
        _ <- scheduleOperations(Seq(spo1, spo2, spo3, spo4))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("02")))
    },
    test("should reject update storage with invalid operation hash") {
      for
        seed <- newSeed
        updateStorage = (spo: SignedPrismOperation, dataHex: String) =>
          builder(seed)
            .updateStorage(spo.getOperationHash.get)
            .bytes(dataHex.decodeHex)
            .build
            .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
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
        spo3 = updateStorage(spo2, "01")
        spo4 = updateStorage(spo2, "02") // invalid operation hash
        _ <- scheduleOperations(Seq(spo1, spo2, spo3, spo4))
        storage1 <- getVdrEntryHex(spo2.getOperationHash.get)
        spo5 = updateStorage(spo3, "03") // points to spo3 as spo4 is invalid
        _ <- scheduleOperations(Seq(spo5))
        storage2 <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage1)(isSome(equalTo("01"))) && assert(storage2)(isSome(equalTo("03")))
    },
    test("should accept update storage with multiple storage entries") {
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
          .bytes("10".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo4 = builder(seed)
          .updateStorage(spo2.getOperationHash.get)
          .bytes("01".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo5 = builder(seed)
          .updateStorage(spo3.getOperationHash.get)
          .bytes("11".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3, spo4, spo5))
        storage1 <- getVdrEntryHex(spo2.getOperationHash.get)
        storage2 <- getVdrEntryHex(spo3.getOperationHash.get)
      yield assert(storage1)(isSome(equalTo("01"))) && assert(storage2)(isSome(equalTo("11")))
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should reject update storage signed with non-VDR key") {
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
          .updateStorage(spo2.getOperationHash.get)
          .bytes("01".decodeHex)
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    },
    test("should reject update storage signed with non-existing key") {
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
          .updateStorage(spo2.getOperationHash.get)
          .bytes("01".decodeHex)
          .build
          .signWith("vdr-1", deriveSecp256k1(seed)("m/0'/8'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    },
    test("should reject update storage signed with removed VDR key") {
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
          .updateDid(spo2.getOperationHash.get, did)
          .removeKey("vdr-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo4 = builder(seed)
          .updateStorage(spo2.getOperationHash.get)
          .bytes("01".decodeHex)
          .build
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3, spo4))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    }
  )
