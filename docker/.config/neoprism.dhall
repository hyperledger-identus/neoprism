let Prelude = (./prelude.dhall).Prelude

let version = (./prelude.dhall).neoPrismVersion

let NeoprismNodeService =
      { Type =
          { image : Text
          , restart : Text
          , ports : List Text
          , command : List Text
          , depends_on : Prelude.Map.Type Text { condition : Text }
          , environment : Prelude.Map.Type Text Text
          , healthcheck :
              { test : List Text
              , interval : Text
              , timeout : Text
              , retries : Natural
              }
          }
      , default =
        { image = "hyperledgeridentus/identus-neoprism:${version}"
        , restart = "always"
        , depends_on = [] : Prelude.Map.Type Text { condition : Text }
        , environment = [] : Prelude.Map.Type Text Text
        , healthcheck =
          { test =
            [ "CMD", "curl", "-f", "http://localhost:8080/api/_system/health" ]
          , interval = "2s"
          , timeout = "5s"
          , retries = 30
          }
        }
      }

let DbSyncDltSourceArgs =
      { Type = { url : Text, pollInterval : Natural }
      , default.pollInterval = 10
      }

let DltSource = < Relay : Text | DbSync : DbSyncDltSourceArgs.Type >

let DltSink =
      { Type =
          { walletHost : Text
          , walletPort : Natural
          , walletId : Text
          , walletPassphrase : Text
          , walletPaymentAddress : Text
          }
      , default = {=}
      }

let Options =
      { Type =
          { hostPort : Natural
          , dbHost : Text
          , network : Text
          , dltSource : DltSource
          , dltSink : Optional DltSink.Type
          , confirmationBlocks : Optional Natural
          , indexInterval : Optional Natural
          , extraDependsOn : List Text
          }
      , default =
        { hostPort = 8080
        , dbHost = "db"
        , network = "mainnet"
        , dltSink = None DltSink.Type
        , confirmationBlocks = None Natural
        , indexInterval = None Natural
        , extraDependsOn = [] : List Text
        }
      }

let makeNodeService =
      \(options : Options.Type) ->
        let mandatoryIndexerNodeEnvs =
              toMap
                { RUST_LOG = "oura=warn,tracing::span=warn,info"
                , NPRISM_DB_URL =
                    "postgres://postgres:postgres@${options.dbHost}:5432/postgres"
                , NPRISM_CARDANO_NETWORK = options.network
                }

        let extraEnvs =
                merge
                  { None = [] : Prelude.Map.Type Text Text
                  , Some =
                      \(n : Natural) ->
                        toMap
                          { NPRISM_CONFIRMATION_BLOCKS = Prelude.Natural.show n
                          }
                  }
                  options.confirmationBlocks
              # merge
                  { None = [] : Prelude.Map.Type Text Text
                  , Some =
                      \(n : Natural) ->
                        toMap { NPRISM_INDEX_INTERVAL = Prelude.Natural.show n }
                  }
                  options.indexInterval
              # merge
                  { Relay =
                      \(addr : Text) ->
                        toMap { NPRISM_CARDANO_RELAY_ADDR = addr }
                  , DbSync =
                      \(args : DbSyncDltSourceArgs.Type) ->
                        toMap
                          { NPRISM_CARDANO_DBSYNC_URL = args.url
                          , NPRISM_CARDANO_DBSYNC_POLL_INTERVAL =
                              Prelude.Natural.show args.pollInterval
                          }
                  }
                  options.dltSource
              # merge
                  { None = [] : Prelude.Map.Type Text Text
                  , Some =
                      \(sink : DltSink.Type) ->
                        let portStr = Prelude.Natural.show sink.walletPort

                        in  toMap
                              { NPRISM_CARDANO_WALLET_BASE_URL =
                                  "http://${sink.walletHost}:${portStr}/v2"
                              , NPRISM_CARDANO_WALLET_WALLET_ID = sink.walletId
                              , NPRISM_CARDANO_WALLET_PASSPHRASE =
                                  sink.walletPassphrase
                              , NPRISM_CARDANO_WALLET_PAYMENT_ADDR =
                                  sink.walletPaymentAddress
                              }
                  }
                  options.dltSink

        let extraDependsOn =
              merge
                { None = [] : Prelude.Map.Type Text { condition : Text }
                , Some =
                    \(sink : DltSink.Type) ->
                      [ { mapKey = sink.walletHost
                        , mapValue.condition = "service_healthy"
                        }
                      ]
                }
                options.dltSink

        let command =
              if    Prelude.Optional.null DltSink.Type options.dltSink
              then  "indexer"
              else  "standalone"

        in  NeoprismNodeService::{
            , ports = [ "${Prelude.Natural.show options.hostPort}:8080" ]
            , environment = mandatoryIndexerNodeEnvs # extraEnvs
            , command = [ command ]
            , depends_on =
                  [ { mapKey = options.dbHost
                    , mapValue.condition = "service_healthy"
                    }
                  ]
                # extraDependsOn
            }

in  { Options, makeNodeService, DltSource, DbSyncDltSourceArgs, DltSink }
