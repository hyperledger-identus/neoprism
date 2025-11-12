package org.hyperledger.identus.prismtest.suite

import org.hyperledger.identus.prismtest.utils.TestUtils
import org.hyperledger.identus.prismtest.NodeName
import proto.prism_ssi.KeyUsage
import zio.test.*
import zio.test.Assertion.*
import zio.ZIO

object CreateDidOperationSuite extends TestUtils:
  def allSpecs = suite("CreateDidOperation")(signatureSpec, publicKeySpec, serviceSpec, vdrSpec, contextSpec)

  private def contextSpec = suite("Context")(
    test("should accept valid context values") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .context("https://www.w3.org/ns/did/v1")
          .context("https://example.com/custom-context")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.context)(
        hasSameElements(Seq("https://www.w3.org/ns/did/v1", "https://example.com/custom-context"))
      )
    },
    test("should reject duplicate context values") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .context("https://example.com/duplicate")
          .context("https://example.com/duplicate")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    }
  )

  private def signatureSpec = suite("Signature")(
    test("should reject non-secp256k1 master key") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY ed25519 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject invalid signing keys") {
      for
        seed <- newSeed
        // key id not exist
        spo1 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-1", deriveSecp256k1(seed)("m/0'/1'/0'"))
        // same key id with wrong private key
        spo2 = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/1'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/1'/1'/1'"))
        _ <- scheduleOperations(Seq(spo1, spo2))
        didData1 <- getDidDocument(spo1.getDid.get)
        didData2 <- getDidDocument(spo2.getDid.get)
      yield assert(didData1)(isNone) && assert(didData2)(isNone)
    }
  )

  private def publicKeySpec = suite("PublicKey")(
    test("should accept only master key") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.context)(isEmpty) &&
        assert(didData.services)(isEmpty) &&
        assert(didData.publicKeys)(hasSize(equalTo(1)))
    },
    test("should accept all supported key types") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("iss-0")(KeyUsage.ISSUING_KEY ed25519 "m/0'/2'/0'")
          .key("comm-0")(KeyUsage.KEY_AGREEMENT_KEY secp256k1 "m/0'/3'/0'")
          .key("auth-0")(KeyUsage.AUTHENTICATION_KEY ed25519 "m/0'/4'/0'")
          .key("revoke-0")(KeyUsage.REVOCATION_KEY secp256k1 "m/0'/5'/0'")
          .key("capinv-0")(KeyUsage.CAPABILITY_INVOCATION_KEY secp256k1 "m/0'/6'/0'")
          .key("capdel-0")(KeyUsage.CAPABILITY_DELEGATION_KEY secp256k1 "m/0'/7'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.context)(isEmpty) &&
        assert(didData.services)(isEmpty) &&
        assert(didData.publicKeys)(hasSize(equalTo(8))) &&
        assert(didData.publicKeys.map(_.usage).distinct)(hasSize(equalTo(8)))
    } @@ NodeName.skipIf("prism-node"),
    test("should reject missing master key") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("auth-0")(KeyUsage.AUTHENTICATION_KEY secp256k1 "m/0'/4'/0'")
          .build
          .signWith("auth-0", deriveSecp256k1(seed)("m/0'/4'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should accept 50 keys") {
      for
        seed <- newSeed
        spo = (0 until 50)
          .foldLeft(builder(seed).createDid) { case (acc, n) =>
            acc.key(s"master-$n")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/$n'")
          }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.publicKeys.length)(equalTo(50))
    },
    test("should reject exceeding maximum key limit (51)") {
      for
        seed <- newSeed
        spo = (0 until 51)
          .foldLeft(builder(seed).createDid) { case (acc, n) =>
            acc.key(s"master-$n")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/$n'")
          }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should accept maximum key ID length (50 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("0" * 50)(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("0" * 50, deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.publicKeys)(hasSize(equalTo(1)))
    },
    test("should reject excessive key ID length (51 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("0" * 51)(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("0" * 51, deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject invalid key ID format") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master 0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("master 0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject empty key ID") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .build
          .signWith("", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject duplicate key IDs") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("duplicate-id")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("duplicate-id")(KeyUsage.ISSUING_KEY secp256k1 "m/0'/1'/1'")
          .build
          .signWith("duplicate-id", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    }
  )

  private def serviceSpec = suite("Service")(
    test("should accept 50 services") {
      for
        seed <- newSeed
        opBuider = builder(seed).createDid
          .key(s"master-0")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/0'")
        spo = (0 until 50)
          .foldLeft(opBuider) { case (acc, n) => acc.service(s"service-$n")("LinkedDomains", "https://example.com") }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.services.length)(equalTo(50))
    },
    test("should reject 51 services") {
      for
        seed <- newSeed
        opBuider = builder(seed).createDid
          .key(s"master-0")(KeyUsage.MASTER_KEY secp256k1 s"m/0'/1'/0'")
        spo = (0 until 51)
          .foldLeft(opBuider) { case (acc, n) => acc.service(s"service-$n")("LinkedDomains", "https://example.com") }
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should accept maximum service ID length (50 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("0" * 50)("LinkedDomains", "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.services)(hasSize(equalTo(1)))
    },
    test("should reject excessive service ID length (51 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("0" * 51)("LinkedDomains", "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject invalid service ID format") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("service 0")("LinkedDomains", "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject empty service ID") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("")("LinkedDomains", "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject duplicate service IDs") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("duplicate-id")("LinkedDomains", "https://example.com/1")
          .service("duplicate-id")("LinkedDomains", "https://example.com/2")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should accept maximum service type length (100 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("service-0")("0" * 100, "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.services)(hasSize(equalTo(1)))
    },
    test("should reject excessive service type length (101 chars)") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("service-0")("0" * 101, "https://example.com")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject invalid service type format") {
      for
        seed <- newSeed
        buildOperation = (serviceType: String) =>
          builder(seed).createDid
            .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
            .service("service-0")(serviceType, "https://example.com")
            .build
            .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spos = Seq(
          buildOperation(""),
          buildOperation(" "),
          buildOperation(" LinkedDomains"),
          buildOperation("LinkedDomains "),
          buildOperation("Linked@Domains"),
          buildOperation("[]"),
          buildOperation("[LinkedDomains ]"),
          buildOperation("[\" LinkedDomains\"]"),
          buildOperation("[\"LinkedDomains \"]"),
          buildOperation("[\"Linked@Domains\"]")
        )
        _ <- scheduleOperations(spos)
        didDataList <- ZIO.foreach(spos) { spo => getDidDocument(spo.getDid.get) }
      yield assert(didDataList)(forall(isNone))
    },
    test("should accept maximum service endpoint length (300 chars)") {
      for
        seed <- newSeed
        serviceEndpoint = s"http://example.com/${"0" * 300}".take(300)
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("service-0")("LinkedDomais", serviceEndpoint)
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get).map(_.get)
      yield assert(didData.services)(hasSize(equalTo(1)))
    },
    test("should reject excessive service endpoint length (301 chars)") {
      for
        seed <- newSeed
        serviceEndpoint = s"http://example.com/${"0" * 300}".take(301)
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .service("service-0")("LinkedDomais", serviceEndpoint)
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    },
    test("should reject invalid service endpoint format") {
      for
        seed <- newSeed
        buildOperation = (serviceEndpoint: String) =>
          builder(seed).createDid
            .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
            .service("service-0")("LinkedDomains", serviceEndpoint)
            .build
            .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        spos = Seq(
          buildOperation(""),
          buildOperation(" "),
          buildOperation(" http://example.com"),
          buildOperation("http://example.com "),
          buildOperation("not a url"),
          buildOperation("[\"not a url\"]"),
          buildOperation("[\" http://example.com\"]"),
          buildOperation("[\"http://example.com \"]"),
          buildOperation("123")
        )
        _ <- scheduleOperations(spos)
        didDataList <- ZIO.foreach(spos) { spo => getDidDocument(spo.getDid.get) }
      yield assert(didDataList)(forall(isNone))
    }
  )

  private def vdrSpec = suite("VDR")(
    test("should reject invalid VDR key type") {
      for
        seed <- newSeed
        spo = builder(seed).createDid
          .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
          .key("vdr-0")(KeyUsage.VDR_KEY ed25519 "m/0'/8'/0'")
          .build
          .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))
        _ <- scheduleOperations(Seq(spo))
        didData <- getDidDocument(spo.getDid.get)
      yield assert(didData)(isNone)
    } @@ NodeName.skipIf("prism-node")
  )
