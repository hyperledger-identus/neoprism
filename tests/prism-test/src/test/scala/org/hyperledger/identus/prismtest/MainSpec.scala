package org.hyperledger.identus.prismtest

import org.hyperledger.identus.prismtest.suite.CreateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.CreateStorageOperationSuite
import org.hyperledger.identus.prismtest.suite.DeactivateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.DeactivateStorageOperationSuite
import org.hyperledger.identus.prismtest.suite.UpdateDidOperationSuite
import org.hyperledger.identus.prismtest.suite.UpdateStorageOperationSuite
import org.hyperledger.identus.prismtest.utils.TestUtils
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

    val neoprismSpec = suite("NeoPRISM suite")(allSpecs)
      .provide(
        Client.default,
        NodeClient.neoprism("localhost", 8080)("localhost", 8090),
        NodeName.layer("neoprism")
      )

    val prismNodeSpec = suite("PRISM node suite")(allSpecs)
      .provide(
        NodeClient.grpc("localhost", 50053),
        NodeName.layer("prism-node")
      )

    // val scalaDidSpec = suite("scala-did node suite")(allSpecs)
    //   .provide(
    //     NodeClient.grpc("localhost", 8980),
    //     NodeName.layer("scala-did")
    //   )

    (neoprismSpec + prismNodeSpec).provide(Runtime.removeDefaultLoggers)
      @@ TestAspect.timed
      @@ TestAspect.withLiveEnvironment
      @@ TestAspect.parallelN(1)
