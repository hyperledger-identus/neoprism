let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "ghcr.io/input-output-hk/prism-node:2.6.1"

let Options =
      { Type =
          { imageOverride : Optional Text
          , nodeDbHost : Text
          , dbSyncDbHost : Text
          , bootstrapTestnetHost : Text
          , walletApiHost : Text
          , walletApiPort : Natural
          , walletPassphrase : Text
          , walletId : Text
          , walletPaymentAddress : Text
          , hostPort : Optional Natural
          , confirmationBlocks : Natural
          }
      , default =
        { imageOverride = None Text
        , walletApiPort = 8090
        , hostPort = None Natural
        , confirmationBlocks = 112
        }
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image = Prelude.Optional.default Text image options.imageOverride
        , ports =
            Prelude.Optional.map
              Natural
              (List Text)
              (\(p : Natural) -> [ "${Prelude.Natural.show p}:50053" ])
              options.hostPort
        , environment = Some
            ( toMap
                { NODE_PSQL_HOST = "${options.nodeDbHost}:5432"
                , NODE_PSQL_DATABASE = "postgres"
                , NODE_PSQL_USERNAME = "postgres"
                , NODE_PSQL_PASSWORD = "postgres"
                , NODE_LEDGER = "cardano"
                , NODE_CARDANO_CONFIRMATION_BLOCKS =
                    Prelude.Natural.show options.confirmationBlocks
                , NODE_REFRESH_AND_SUBMIT_PERIOD = "1s"
                , NODE_MOVE_SCHEDULED_TO_PENDING_PERIOD = "1s"
                , NODE_SCHEDULE_SYNC_PERIOD = "1s"
                , NODE_CARDANO_NETWORK = "testnet"
                , NODE_CARDANO_WALLET_PASSPHRASE = options.walletPassphrase
                , NODE_CARDANO_WALLET_ID = options.walletId
                , NODE_CARDANO_PAYMENT_ADDRESS = options.walletPaymentAddress
                , NODE_CARDANO_WALLET_API_HOST = options.walletApiHost
                , NODE_CARDANO_WALLET_API_PORT =
                    Prelude.Natural.show options.walletApiPort
                , NODE_CARDANO_PRISM_GENESIS_BLOCK = "0"
                , NODE_CARDANO_DB_SYNC_HOST = "${options.dbSyncDbHost}:5432"
                , NODE_CARDANO_DB_SYNC_DATABASE = "postgres"
                , NODE_CARDANO_DB_SYNC_USERNAME = "postgres"
                , NODE_CARDANO_DB_SYNC_PASSWORD = "postgres"
                }
            )
        , depends_on = Some
          [ docker.ServiceCondition.healthy options.nodeDbHost
          , docker.ServiceCondition.healthy options.dbSyncDbHost
          , docker.ServiceCondition.healthy options.walletApiHost
          , docker.ServiceCondition.completed options.bootstrapTestnetHost
          ]
        }

in  { Options, mkService }
