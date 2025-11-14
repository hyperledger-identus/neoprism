package org.hyperledger.identus.prismtest

import io.grpc.ManagedChannelBuilder
import io.grpc.StatusRuntimeException
import io.iohk.atala.prism.protos.node_api.DIDData
import io.iohk.atala.prism.protos.node_api.GetDidDocumentRequest
import io.iohk.atala.prism.protos.node_api.GetOperationInfoRequest
import io.iohk.atala.prism.protos.node_api.NodeServiceGrpc
import io.iohk.atala.prism.protos.node_api.NodeServiceGrpc.NodeService
import io.iohk.atala.prism.protos.node_api.OperationOutput.OperationMaybe
import io.iohk.atala.prism.protos.node_api.OperationStatus
import io.iohk.atala.prism.protos.node_api.ScheduleOperationsRequest
import monocle.syntax.all.*
import org.hyperledger.identus.prismtest.utils.CryptoUtils
import org.hyperledger.identus.prismtest.utils.ProtoUtils
import proto.prism.SignedPrismOperation
import zio.*
import zio.http.*
import zio.json.*
import zio.schema.codec.JsonCodec.zioJsonBinaryCodec

import scala.language.implicitConversions

type OperationRef = String

object Errors:
  case class BadRequest()

trait NodeClient:
  def scheduleOperations(operations: Seq[SignedPrismOperation]): IO[Errors.BadRequest, Seq[OperationRef]]
  def isOperationConfirmed(ref: OperationRef): UIO[Boolean]
  def getDidDocument(did: String): UIO[Option[DIDData]]

object NodeClient:

  def grpc(host: String, port: Int): TaskLayer[NodeClient] =
    ZLayer.scoped(
      ZIO
        .acquireRelease(
          ZIO.attempt(ManagedChannelBuilder.forAddress(host, port).usePlaintext.build)
        )(channel => ZIO.attempt(channel.shutdown()).orDie)
        .map(NodeServiceGrpc.stub(_))
        .map(GrpcNodeClient(_))
    )

  def neoprism(
      neoprismHost: String,
      neoprismPort: Int
  )(cardanoWalletHost: String, cardanoWalletPort: Int)(skipCheckMillis: Option[Int]): RLayer[Client, NodeClient] =
    ZLayer
      .fromZIO {
        for
          neoprismClient <- ZIO.serviceWith[Client](_.url(url"http://$neoprismHost:$neoprismPort"))
          cardanoWalletClient <- ZIO.serviceWith[Client](_.url(url"http://$cardanoWalletHost:$cardanoWalletPort"))
        yield NeoprismNodeClient(neoprismClient, cardanoWalletClient, skipCheckMillis)
      }

private class GrpcNodeClient(nodeService: NodeService) extends NodeClient, CryptoUtils, ProtoUtils:

  override def scheduleOperations(operations: Seq[SignedPrismOperation]): IO[Errors.BadRequest, Seq[OperationRef]] =
    ZIO
      .fromFuture(_ => nodeService.scheduleOperations(ScheduleOperationsRequest(signedOperations = operations)))
      .flatMap(response =>
        ZIO.foreach(response.outputs.map(_.operationMaybe)) {
          case OperationMaybe.OperationId(id) => ZIO.succeed(id.toByteArray().toHexString)
          case _                              => ZIO.dieMessage("operation unsuccessful")
        }
      )
      .catchAll {
        case s: StatusRuntimeException if s.getStatus.getCode.toStatus() == io.grpc.Status.INVALID_ARGUMENT =>
          ZIO.fail(Errors.BadRequest())
        case e => ZIO.die(e)
      }

  override def isOperationConfirmed(ref: OperationRef): UIO[Boolean] =
    ZIO
      .fromFuture(_ => nodeService.getOperationInfo(GetOperationInfoRequest(ref.decodeHex)))
      .map(_.operationStatus match
        case OperationStatus.CONFIRMED_AND_APPLIED  => true
        case OperationStatus.CONFIRMED_AND_REJECTED => true
        case _                                      => false)
      .orDie

  override def getDidDocument(did: String): UIO[Option[DIDData]] =
    ZIO
      .fromFuture(_ => nodeService.getDidDocument(GetDidDocumentRequest(did = did)))
      .orDie
      .map(_.document)
      .map(
        _.map(didData =>
          didData
            .focus(_.publicKeys)
            .modify(_.filter(_.unknownFields.getField(6).isEmpty)) // remove revoked entry
            .focus(_.services)
            .modify(_.filter(_.unknownFields.getField(6).isEmpty)) // remove revoked entry
        )
      )

private class NeoprismNodeClient(neoprismClient: Client, cardanoWalletClient: Client, skipCheckMillis: Option[Int])
    extends NodeClient,
      CryptoUtils:

  import NeoprismNodeClient.*

  override def scheduleOperations(operations: Seq[SignedPrismOperation]): IO[Errors.BadRequest, Seq[OperationRef]] =
    val requestBody = ScheduleOperationRequest(signed_operations = operations.map(_.toByteArray.toHexString))
    neoprismClient.batched
      .post("/api/signed-operation-submissions")(Body.from(requestBody).contentType(MediaType.application.json))
      .flatMap(resp => resp.body.to[ScheduleOperationResponse])
      .map(resp => Seq(resp.tx_id))
      .orDie

  override def isOperationConfirmed(ref: OperationRef): UIO[Boolean] =
    skipCheckMillis match
      case Some(delay) => ZIO.succeed(true).delay(delay.millis)
      case None        =>
        for
          wallets <- cardanoWalletClient.batched
            .get("/v2/wallets")
            .flatMap(_.body.to[List[Wallet]])
            .orDie
          wallet <- ZIO.succeed(wallets.headOption).someOrFailException.orDie
          txSlotNo <- cardanoWalletClient.batched
            .get(url"/v2/wallets/${wallet.id}/transactions/$ref".toString)
            .flatMap(_.body.to[TransactionResponse])
            .map(_.inserted_at.map(_.absolute_slot_number))
            .orDie
          indexerSlotNo <- neoprismClient.batched
            .get(url"/api/indexer-stats".toString)
            .flatMap(_.body.to[IndexerStatsResponse])
            .map(_.last_prism_slot_number)
            .orDie
        yield indexerSlotNo.exists(indexerSlot => txSlotNo.exists(txSlot => indexerSlot >= txSlot))

  override def getDidDocument(did: String): UIO[Option[DIDData]] =
    for
      resp <- neoprismClient.batched.get(url"/api/did-data/$did".toString).orDie
      body <- resp.body.asString.orDie
      didData <- resp.status match
        case Status.NotFound => ZIO.none
        case Status.Ok       => ZIO.some(DIDData.parseFrom(body.decodeHex))
        case _               => ZIO.dieMessage("Could not get DIDData")
    yield didData

private object NeoprismNodeClient:

  case class ScheduleOperationRequest(signed_operations: Seq[String])

  object ScheduleOperationRequest:
    given dec: JsonDecoder[ScheduleOperationRequest] = JsonDecoder.derived
    given enc: JsonEncoder[ScheduleOperationRequest] = JsonEncoder.derived
    given JsonCodec[ScheduleOperationRequest] = JsonCodec.fromEncoderDecoder(enc, dec)

  case class ScheduleOperationResponse(tx_id: String)

  object ScheduleOperationResponse:
    given dec: JsonDecoder[ScheduleOperationResponse] = JsonDecoder.derived
    given enc: JsonEncoder[ScheduleOperationResponse] = JsonEncoder.derived
    given JsonCodec[ScheduleOperationResponse] = JsonCodec.fromEncoderDecoder(enc, dec)

  case class Wallet(id: String)

  object Wallet:
    given dec: JsonDecoder[Wallet] = JsonDecoder.derived
    given enc: JsonEncoder[Wallet] = JsonEncoder.derived
    given JsonCodec[Wallet] = JsonCodec.fromEncoderDecoder(enc, dec)

  case class TransactionResponse(inserted_at: Option[LedgerTimestamp])

  object TransactionResponse:
    given dec: JsonDecoder[TransactionResponse] = JsonDecoder.derived
    given enc: JsonEncoder[TransactionResponse] = JsonEncoder.derived
    given JsonCodec[TransactionResponse] = JsonCodec.fromEncoderDecoder(enc, dec)

  case class LedgerTimestamp(absolute_slot_number: Int)

  object LedgerTimestamp:
    given dec: JsonDecoder[LedgerTimestamp] = JsonDecoder.derived
    given enc: JsonEncoder[LedgerTimestamp] = JsonEncoder.derived
    given JsonCodec[LedgerTimestamp] = JsonCodec.fromEncoderDecoder(enc, dec)

  case class IndexerStatsResponse(last_prism_block_number: Option[Int], last_prism_slot_number: Option[Int])

  object IndexerStatsResponse:
    given dec: JsonDecoder[IndexerStatsResponse] = JsonDecoder.derived
    given enc: JsonEncoder[IndexerStatsResponse] = JsonEncoder.derived
    given JsonCodec[IndexerStatsResponse] = JsonCodec.fromEncoderDecoder(enc, dec)
