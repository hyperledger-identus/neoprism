let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "blockfrost/backend-ryo:v4.3.0"

let DbSyncDbArgs =
      { Type =
          { host : Text
          , port : Natural
          , dbName : Text
          , username : Text
          , password : Text
          }
      , default = {=}
      }

let Options =
      { Type =
          { hostPort : Optional Natural
          , dbsyncDb : DbSyncDbArgs.Type
          , network : Text
          , testnetVolume : Text
          , configFile : Text
          , bootstrapTestnetHost : Optional Text
          }
      , default =
        { hostPort = None Natural
        , network = "mainnet"
        , bootstrapTestnetHost = None Text
        }
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image
        , ports =
            Prelude.Optional.map
              Natural
              (List Text)
              (\(p : Natural) -> [ "${Prelude.Natural.show p}:3000" ])
              options.hostPort
        , environment = Some
            ( toMap
                { BLOCKFROST_CONFIG_DBSYNC_HOST = options.dbsyncDb.host
                , BLOCKFROST_CONFIG_DBSYNC_PORT =
                    Prelude.Natural.show options.dbsyncDb.port
                , BLOCKFROST_CONFIG_DBSYNC_DATABASE = options.dbsyncDb.dbName
                , BLOCKFROST_CONFIG_DBSYNC_USER = options.dbsyncDb.username
                , BLOCKFROST_CONFIG_DBSYNC_PASSWORD = options.dbsyncDb.password
                , BLOCKFROST_CONFIG_NETWORK = options.network
                , BLOCKFROST_CONFIG_GENESIS_DATA_FOLDER = "/node/testnet"
                , BLOCKFROST_MITHRIL_ENABLED = "false"
                , NODE_ENV = "development"
                }
            )
        , volumes = Some
          [ "${options.testnetVolume}:/node/testnet"
          , "${options.configFile}:/app/config/development.yaml"
          ]
        , depends_on = Some
            (   [ docker.ServiceCondition.healthy options.dbsyncDb.host ]
              # merge
                  { None = [] : List docker.ServiceCondition.Type
                  , Some =
                      \(host : Text) ->
                        [ docker.ServiceCondition.completed host ]
                  }
                  options.bootstrapTestnetHost
            )
        }

in  { mkService, Options, DbSyncDbArgs }
