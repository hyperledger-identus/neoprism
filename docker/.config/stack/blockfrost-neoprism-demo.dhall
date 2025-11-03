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
          , dbsyncPort : Text
          , dbsyncDb : Text
          , dbsyncUser : Text
          , dbsyncPassword : Text
          , network : Text
          , testnetVolume : Text
          }
      , default =
        { dbsyncUrl = "\${DBSYNC_URL}"
        , dbsyncHost = "\${DBSYNC_HOST}"
        , dbsyncPort = "\${DBSYNC_PORT:-5432}"
        , dbsyncDb = "\${DBSYNC_DB:-postgres}"
        , dbsyncUser = "\${DBSYNC_USER:-postgres}"
        , dbsyncPassword = "\${DBSYNC_PASSWORD}"
        , network = "\${NETWORK:-mainnet}"
        , testnetVolume = "\${TESTNET_VOLUME:-node-testnet}"
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
                    , waitForDbSync = False
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
