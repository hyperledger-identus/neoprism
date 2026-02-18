package org.hyperledger.identus.prismtest.suite

import org.hyperledger.identus.prismtest.utils.TestUtils
import org.hyperledger.identus.prismtest.NodeClient
import zio.URIO

trait StorageTestUtils extends TestUtils:
  protected def getVdrEntryHex(initOperationHash: Array[Byte]): URIO[NodeClient, Option[String]] =
    zio.ZIO
      .serviceWithZIO[NodeClient](_.getVdrEntry(initOperationHash))
      .map(_.map(_.toHexString))
