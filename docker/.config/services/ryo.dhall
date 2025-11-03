let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "blockfrost/backend-ryo:v4.3.0"

let DbSyncDbArgs =
      { Type =
          { host : Text
          , port : Text
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
          , testnetVolume : Optional Text
          , configFile : Text
          , bootstrapTestnetHost : Optional Text
          , waitForDbSync : Bool
          }
      , default =
        { hostPort = None Natural
        , network = "mainnet"
        , testnetVolume = None Text
        , bootstrapTestnetHost = None Text
        , waitForDbSync = True
        }
      }

let mkService =
      \(options : Options.Type) ->
        let testnetVolumeMount =
              merge
                { None = [] : List Text
                , Some = \(vol : Text) -> [ "${vol}:/node/testnet" ]
                }
                options.testnetVolume

        let configVolume =
              [ "${options.configFile}:/app/config/development.yaml" ]

        let allVolumes = testnetVolumeMount # configVolume

        in  docker.Service::{
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
                    , BLOCKFROST_CONFIG_DBSYNC_PORT = options.dbsyncDb.port
                    , BLOCKFROST_CONFIG_DBSYNC_DATABASE =
                        options.dbsyncDb.dbName
                    , BLOCKFROST_CONFIG_DBSYNC_USER = options.dbsyncDb.username
                    , BLOCKFROST_CONFIG_DBSYNC_PASSWORD =
                        options.dbsyncDb.password
                    , BLOCKFROST_CONFIG_NETWORK = options.network
                    , BLOCKFROST_CONFIG_GENESIS_DATA_FOLDER = "/node/testnet"
                    , BLOCKFROST_MITHRIL_ENABLED = "false"
                    , NODE_ENV = "development"
                    }
                )
            , volumes = Some allVolumes
            , depends_on =
                let dbSyncCondition =
                      if    options.waitForDbSync
                      then  [ docker.ServiceCondition.healthy
                                options.dbsyncDb.host
                            ]
                      else  [] : List docker.ServiceCondition.Type

                let testnetCondition =
                      merge
                        { None = [] : List docker.ServiceCondition.Type
                        , Some =
                            \(host : Text) ->
                              [ docker.ServiceCondition.completed host ]
                        }
                        options.bootstrapTestnetHost

                in  Some (dbSyncCondition # testnetCondition)
            }

in  { mkService, Options, DbSyncDbArgs }
