package org.hyperledger.identus.prismtest.utils

import com.google.protobuf.ByteString
import io.iohk.atala.prism.protos.node_api.DIDData
import monocle.syntax.all.*
import org.hyperledger.identus.apollo.derivation.EdHDKey
import org.hyperledger.identus.apollo.derivation.HDKey
import org.hyperledger.identus.apollo.utils.ByteArrayExtKt
import org.hyperledger.identus.apollo.utils.KMMEdPrivateKey
import org.hyperledger.identus.apollo.utils.StringExtKt
import org.hyperledger.identus.prismtest.Errors
import org.hyperledger.identus.prismtest.NodeClient
import org.hyperledger.identus.prismtest.OperationRef
import proto.prism.PrismOperation
import proto.prism.PrismOperation.Operation
import proto.prism.SignedPrismOperation
import proto.prism_ssi.AddKeyAction
import proto.prism_ssi.CompressedECKeyData
import proto.prism_ssi.KeyUsage
import proto.prism_ssi.ProtoCreateDID
import proto.prism_ssi.ProtoCreateDID.DIDCreationData
import proto.prism_ssi.ProtoDeactivateDID
import proto.prism_ssi.ProtoUpdateDID
import proto.prism_ssi.PublicKey
import proto.prism_ssi.PublicKey.KeyData
import proto.prism_ssi.RemoveKeyAction
import proto.prism_ssi.Service
import proto.prism_ssi.UpdateDIDAction
import proto.prism_ssi.UpdateDIDAction.Action
import proto.prism_storage.ProtoCreateStorageEntry
import proto.prism_storage.ProtoDeactivateStorageEntry
import proto.prism_storage.ProtoUpdateStorageEntry
import zio.*

import scala.language.implicitConversions

trait TestUtils extends CryptoUtils, ProtoUtils, TestDsl

trait TestDsl extends ProtoUtils, CryptoUtils:
  def newSeed: UIO[Array[Byte]] = Random.nextBytes(64).map(_.toArray)
  def builder(seed: Array[Byte]): OpBuilder = OpBuilder(seed)

  def getDidDocument(did: String): URIO[NodeClient, Option[DIDData]] =
    ZIO.serviceWithZIO[NodeClient](_.getDidDocument(did))

  def scheduleOperations(
      operations: Seq[SignedPrismOperation],
      batch: Boolean = true
  ): URIO[NodeClient, Seq[OperationRef]] =
    val batched = if batch then Seq(operations) else operations.grouped(1).toList
    ZIO
      .foreach(batched) { ops =>
        ZIO
          .serviceWithZIO[NodeClient](nodeClient => nodeClient.scheduleOperations(ops))
          .catchAll {
            // When node respond with BadRequest, we assume it does not return any OperationRef.
            // This is needed to normalize the node behavior to provide uniform API accross all NodeClient
            // as some implementation eagerly reject the invalid operation on submission.
            // TODO: change the write path so that it does not reject so we can test indexing logic better
            case _: Errors.BadRequest => ZIO.succeed(Nil)
          }
      }
      .map(_.flatten)
      .tap(waitUntilConfirmed)

  private def waitUntilConfirmed(operationRefs: Seq[OperationRef]): URIO[NodeClient, Unit] =
    ZIO
      .foreach(operationRefs) { operationRef =>
        ZIO.serviceWithZIO[NodeClient](nodeClient =>
          ZIO.logInfo(s"waiting for operation $operationRef to be confirmed") *>
            nodeClient
              .isOperationConfirmed(operationRef)
              .filterOrFail(identity)(Exception("operation is not confirmed"))
              .retry(Schedule.spaced(200.millis))
              .timeout(60.seconds)
              .orDie
        )
      }
      .unit

  extension (ku: KeyUsage)
    def secp256k1(path: String): Array[Byte] => (KeyUsage, HDKey) = (seed: Array[Byte]) =>
      ku -> deriveSecp256k1(seed)(path)
    def ed25519(path: String): Array[Byte] => (KeyUsage, EdHDKey) = (seed: Array[Byte]) =>
      ku -> deriveEd25519(seed)(path)

  extension (op: SignedPrismOperation)
    def getDid: Option[String] = op.operation.flatMap(_.getDid)
    def getOperationHash: Option[Array[Byte]] = op.operation.map(_.getOperationHash)

  extension (op: PrismOperation)
    def getOperationHash: Array[Byte] = sha256(op.toByteArray)

    def getDid: Option[String] =
      op.operation match
        case Operation.CreateDid(_) =>
          val bytes = op.toByteArray
          val hexSuffix = sha256(bytes).toHexString
          Some(s"did:prism:$hexSuffix")
        case _ => None

    def signWith(keyId: String, hdKey: HDKey): SignedPrismOperation =
      SignedPrismOperation(
        signedWith = keyId,
        signature = hdKey.getKMMSecp256k1PrivateKey().sign(op.toByteArray),
        operation = Some(op)
      )

  case class OpBuilder(seed: Array[Byte]):
    def createDid: CreateDidOpBuilder =
      CreateDidOpBuilder(seed, ProtoCreateDID(didData = Some(DIDCreationData())))

    def updateDid(prevOperationHash: Array[Byte], did: String): UpdateDidOpBuilder =
      UpdateDidOpBuilder(seed, ProtoUpdateDID(prevOperationHash, did.replace("did:prism:", "")))

    def deactivateDid(prevOperationHash: Array[Byte], did: String): PrismOperation =
      val op = ProtoDeactivateDID(prevOperationHash, id = did.replace("did:prism:", ""))
      PrismOperation(Operation.DeactivateDid(op))

    def createStorage(did: String, nonce: Array[Byte] = Array.empty): CreateStorageBuilder =
      CreateStorageBuilder(
        seed,
        ProtoCreateStorageEntry(didPrismHash = did.replace("did:prism:", "").decodeHex, nonce = nonce)
      )

    def updateStorage(prevOperationHash: Array[Byte]): UpdateStorageBuilder =
      UpdateStorageBuilder(
        seed,
        ProtoUpdateStorageEntry(previousEventHash = prevOperationHash)
      )

    def deactivateStorage(prevOperationHash: Array[Byte]): PrismOperation =
      val op = ProtoDeactivateStorageEntry(prevOperationHash)
      PrismOperation(Operation.DeactivateStorageEntry(op))

  case class CreateDidOpBuilder(seed: Array[Byte], op: ProtoCreateDID):
    def build: PrismOperation = PrismOperation(Operation.CreateDid(op))

    def key(keyId: String)(makeKey: Array[Byte] => (KeyUsage, HDKey | EdHDKey)): CreateDidOpBuilder =
      val (keyUsage, hdKey) = makeKey(seed)
      this
        .focus(_.op.didData.some.publicKeys)
        .modify(_ :+ PublicKey(id = keyId, usage = keyUsage, keyData = hdKey))

    def context(value: String): CreateDidOpBuilder =
      this
        .focus(_.op.didData.some.context)
        .modify(_ :+ value)

    def service(serviceId: String)(serviceType: String, serviceEndpoint: String): CreateDidOpBuilder =
      this
        .focus(_.op.didData.some.services)
        .modify(
          _ :+ Service(
            id = serviceId,
            `type` = serviceType,
            serviceEndpoint = serviceEndpoint
          )
        )

  case class UpdateDidOpBuilder(seed: Array[Byte], op: ProtoUpdateDID):
    def build: PrismOperation = PrismOperation(Operation.UpdateDid(op))

    def addKey(keyId: String)(makeKey: Array[Byte] => (KeyUsage, HDKey | EdHDKey)): UpdateDidOpBuilder =
      val (keyUsage, hdKey) = makeKey(seed)
      this
        .focus(_.op.actions)
        .modify(
          _ :+ UpdateDIDAction(
            Action.AddKey(AddKeyAction(Some(PublicKey(id = keyId, usage = keyUsage, keyData = hdKey))))
          )
        )

    def removeKey(keyId: String): UpdateDidOpBuilder =
      this
        .focus(_.op.actions)
        .modify(_ :+ UpdateDIDAction(Action.RemoveKey(RemoveKeyAction(keyId))))

    def patchContext(context: Seq[String]): UpdateDidOpBuilder =
      this
        .focus(_.op.actions)
        .modify(_ :+ UpdateDIDAction(Action.PatchContext(proto.prism_ssi.PatchContextAction(context))))

  case class CreateStorageBuilder(seed: Array[Byte], op: ProtoCreateStorageEntry):
    def build: PrismOperation = PrismOperation(Operation.CreateStorageEntry(op))

    def bytes(b: Array[Byte]): CreateStorageBuilder =
      this
        .focus(_.op.data)
        .modify(_ => ProtoCreateStorageEntry.Data.Bytes(b))

  case class UpdateStorageBuilder(seed: Array[Byte], op: ProtoUpdateStorageEntry):
    def build: PrismOperation = PrismOperation(Operation.UpdateStorageEntry(op))

    def bytes(b: Array[Byte]): UpdateStorageBuilder =
      this
        .focus(_.op.data)
        .modify(_ => ProtoUpdateStorageEntry.Data.Bytes(b))

trait ProtoUtils extends CryptoUtils:
  given Conversion[Array[Byte], ByteString] = ByteString.copyFrom

  given Conversion[HDKey | EdHDKey, KeyData] = (hdKey: HDKey | EdHDKey) =>
    hdKey match
      case hdKey: HDKey =>
        KeyData.CompressedEcKeyData(
          CompressedECKeyData(
            curve = "secp256k1",
            data = hdKey.getKMMSecp256k1PrivateKey().getPublicKey().getCompressed()
          )
        )
      case hdKey: EdHDKey =>
        val privKey = KMMEdPrivateKey(hdKey.getPrivateKey())
        KeyData.CompressedEcKeyData(
          CompressedECKeyData(
            curve = "Ed25519",
            data = privKey.publicKey().getRaw()
          )
        )

trait CryptoUtils:
  extension (str: String) def decodeHex: Array[Byte] = StringExtKt.decodeHex(str)
  extension (bytes: Array[Byte]) def toHexString: String = ByteArrayExtKt.toHexString(bytes)

  def sha256(bytes: Array[Byte]): Array[Byte] = Sha256Hash.compute(bytes).bytes.toArray

  def deriveSecp256k1(seed: Array[Byte])(pathStr: String): HDKey =
    HDKey(seed, 0, 0).derive(pathStr)

  def deriveEd25519(seed: Array[Byte])(pathStr: String): EdHDKey =
    EdHDKey.Companion.initFromSeed(seed).derive(pathStr)
