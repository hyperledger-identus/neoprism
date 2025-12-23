package org.hyperledger.identus.prismtest

import org.hyperledger.identus.prismtest.suite.CreateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.CreateStorageOperationSuite
import org.hyperledger.identus.prismtest.suite.DeactivateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.DeactivateStorageOperationSuite
import org.hyperledger.identus.prismtest.suite.UpdateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.UpdateStorageOperationSuite
import org.hyperledger.identus.prismtest.utils.TestUtils
import proto.prism.PrismBlock
import proto.prism.PrismObject
import proto.prism_ssi.KeyUsage
import zio.*
import zio.http.Client
import zio.test.*

object MainSpec extends ZIOSpecDefault, TestUtils:

  override def spec =
    val allSpecs =
      CreateDidOperationSuite.allSpecs +
        UpdateDidOperationSuite.allSpecs +
        DeactivateDidOperationSuite.allSpecs +
        CreateStorageOperationSuite.allSpecs +
        UpdateStorageOperationSuite.allSpecs +
        DeactivateStorageOperationSuite.allSpecs

    val neoprismLayer = NodeClient.neoprism("localhost", 18080)

    val neoprismSpec = suite("NeoPRISM suite")(allSpecs)
      .provide(
        Client.default,
        neoprismLayer,
        NodeName.layer("neoprism")
      )

    // val prismNodeSpec = suite("PRISM node suite")(allSpecs)
    //   .provide(
    //     NodeClient.grpc("localhost", 50053),
    //     NodeName.layer("prism-node")
    //   )

    (neoprismSpec + generateDidFixtureSpec).provide(Runtime.removeDefaultLoggers)
      @@ TestAspect.timed
      @@ TestAspect.withLiveEnvironment
      @@ TestAspect.parallelN(1)

  // Comment the ignore aspect and run `sbt testOnly -- -tags fixture`
  // to output the generated test vector
  private def generateDidFixtureSpec = test("generate did fixtures for testing") {
    val seed = Array.fill[Byte](64)(0)
    val vdrKeyName = "vdr-0"
    val makeVdrKey = KeyUsage.VDR_KEY secp256k1 "m/0'/8'/0'"

    val spo = builder(seed).createDid
      .key("master-0")(KeyUsage.MASTER_KEY secp256k1 "m/0'/1'/0'")
      .key(vdrKeyName)(makeVdrKey)
      .build
      .signWith("master-0", deriveSecp256k1(seed)("m/0'/1'/0'"))

    val did = spo.getDid.get
    val (_, vdrHdKey) = makeVdrKey(seed)
    val vdrPrivateKeyHex = vdrHdKey.getKMMSecp256k1PrivateKey().getEncoded().toHexString
    val prismObjectHex = PrismObject(blockContent = Some(PrismBlock(operations = Seq(spo)))).toByteArray.toHexString

    for
      _ <- ZIO.debug(s"DID                : $did")
      _ <- ZIO.debug(s"VDR key name       : $vdrKeyName")
      _ <- ZIO.debug(s"VDR privateKey hex : $vdrPrivateKeyHex")
      _ <- ZIO.debug(s"PrismObject hex    : $prismObjectHex")
    yield assertCompletes
  } @@ TestAspect.tag("fixture") @@ TestAspect.ignore
