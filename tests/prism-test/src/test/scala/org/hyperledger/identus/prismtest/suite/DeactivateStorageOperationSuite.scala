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
  ) @@ NodeName.skipIf("prism-node", "scala-did")

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
        storage1 <- getDidDocument(did).map(_.get).map(extractStorageHex)
        spo3 = builder(seed)
          .deactivateStorage(spo2.getOperationHash.get)
          .signWith("vdr-0", deriveSecp256k1(seed)("m/0'/8'/0'"))
        _ <- scheduleOperations(Seq(spo3, spo2))
        storage2 <- getDidDocument(did).map(_.get).map(extractStorageHex)
      yield assert(storage1)(hasSameElements(Seq("00"))) && assert(storage2)(isEmpty)
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
        storage1 <- getDidDocument(did).map(_.get).map(extractStorageHex)
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
        didData <- getDidDocument(did).map(_.get)
        storage2 = extractStorageHex(didData)
      yield assert(storage1)(hasSameElements(Seq("00", "10"))) &&
        assert(storage2)(isEmpty) &&
        assert(didData.publicKeys.map(_.id))(hasSameElements(Seq("master-0")))
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
        storage <- getDidDocument(did).map(_.get).map(extractStorageHex)
      yield assert(storage)(hasSameElements(Seq("00")))
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
        didData <- getDidDocument(did).map(_.get)
        storage = extractStorageHex(didData)
      yield assert(storage)(isEmpty) && assert(didData.publicKeys)(hasSize(equalTo(2)))
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
        storage <- getDidDocument(did).map(_.get).map(extractStorageHex)
      yield assert(storage)(hasSameElements(Seq("00")))
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
        storage <- getDidDocument(did).map(_.get).map(extractStorageHex)
      yield assert(storage)(hasSameElements(Seq("00")))
    }
  )
