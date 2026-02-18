package org.hyperledger.identus.prismtest.suite

import org.hyperledger.identus.prismtest.NodeName
import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*

object DeactivateStorageOperationSuite extends StorageTestUtils:

  def allSpecs = suite("DeactivateStorageOperation")(
    signatureSpec,
    prevOperationHashSpec,
    deactivatedStorageSpec
  ) @@ NodeName.skipIf("prism-node")

  private def deactivatedStorageSpec = suite("Deactivated storage")(
    test("should reject resubmitting same create storage operation after deactivation") {
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
        storage1 <- getVdrEntryHex(spo2.getOperationHash.get)
        spo3 = builder(seed)
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo3, spo2))
        storage2 <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage1)(isSome(equalTo("00"))) && assert(storage2)(isNone)
    }
  )

  private def prevOperationHashSpec = suite("PreviousOperationHash")(
    test("should accept multiple storage deactivations") {
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
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage1_1 <- getVdrEntryHex(spo2.getOperationHash.get)
        storage1_2 <- getVdrEntryHex(spo3.getOperationHash.get)
        spo4 = builder(seed)
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        spo5 = builder(seed)
          .deactivateStorage(spo3.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        // for assertion that prevOperationHash is the latest one (spo5)
        spo6 = builder(seed)
          .updateDid(spo5.getOperationHash.get, did)
          .removeKey("vdr-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo4, spo5, spo6))
        storage2_1 <- getVdrEntryHex(spo2.getOperationHash.get)
        storage2_2 <- getVdrEntryHex(spo3.getOperationHash.get)
        didData <- getDidDocument(did)
      yield assert(storage1_1)(isSome(equalTo("00"))) &&
        assert(storage1_2)(isSome(equalTo("10"))) &&
        assert(storage2_1)(isNone) &&
        assert(storage2_2)(isNone) &&
        assert(didData.get.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
    },
    test("should reject with invalid operation hash") {
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
          .deactivateStorage(spo1.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should accept when signed with active VDR key") {
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
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
        didData <- getDidDocument(did)
      yield assert(storage)(isNone) && assert(didData.get.publicKeys)(hasSize(equalTo(2)))
    },
    test("should reject when signed with non-existing VDR key") {
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
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-1", deriveSecp256k1(seed)("m/0'/8'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    },
    test("should reject when signed with removed VDR key") {
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
          .updateDid(spo2.getOperationHash.get, did)
          .removeKey("vdr-0")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spo4 = builder(seed)
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo1, spo2, spo3, spo4))
        storage <- getVdrEntryHex(spo2.getOperationHash.get)
      yield assert(storage)(isSome(equalTo("00")))
    }
  )
