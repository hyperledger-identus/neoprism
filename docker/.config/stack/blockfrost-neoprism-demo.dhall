let neoprismVersion = (../prelude.dhall).neoprismVersion

let neoprism = ../services/neoprism.dhall

let docker = ../docker.dhall

let db = ../services/db.dhall

let ryo = ../services/ryo.dhall

let caddy = ../services/caddy.dhall

let Options =
      { Type =
          { dbsyncUrl : Text
          , dbsyncHost : Text
          , dbsyncPort : Natural
          , dbsyncDb : Text
          , dbsyncUser : Text
          , dbsyncPassword : Text
          , network : Text
          , testnetVolume : Text
          }
      , default =
        { dbsyncUrl = "<REMOTE_DBSYNC_URL>"
        , dbsyncHost = "<REMOTE_DBSYNC_HOST>"
        , dbsyncPort = 5432
        , dbsyncDb = "postgres"
        , dbsyncUser = "postgres"
        , dbsyncPassword = "<REMOTE_DBSYNC_PASSWORD>"
        , network = "mainnet"
        , testnetVolume = "node-testnet"
        }
      }

let mkStack =
      \(options : Options.Type) ->
        let services =
              { neoprism =
                  neoprism.mkService
                    neoprism.Options::{
                    , dbHost = "db-neoprism"
                    , network = options.network
                    , dltSource =
                        neoprism.DltSource.DbSync
                          neoprism.DbSyncDltSourceArgs::{
                          , url = options.dbsyncUrl
                          , pollInterval = 10
                          }
                    }
              , db-neoprism = db.mkService db.Options::{=}
              , bf-ryo =
                  ryo.mkService
                    ryo.Options::{
                    , dbsyncDb = ryo.DbSyncDbArgs::{
                      , host = options.dbsyncHost
                      , port = options.dbsyncPort
                      , dbName = options.dbsyncDb
                      , username = options.dbsyncUser
                      , password = options.dbsyncPassword
                      }
                    , network = options.network
                    , testnetVolume = options.testnetVolume
                    , configFile = "./ryo.yaml"
                    , bootstrapTestnetHost = None Text
                    }
              , caddy =
                  caddy.mkService
                    caddy.Options::{
                    , hostPort = Some 18082
                    , targetPort = 3000
                    , caddyfile = "./Caddyfile-blockfrost"
                    }
              }

        in  { services, volumes = toMap { node-testnet = {=} } }

in  { mkStack, Options }
