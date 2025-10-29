let neoprismVersion = (../prelude.dhall).neoprismVersion

let neoprism = ../services/neoprism.dhall

let docker = ../docker.dhall

let db = ../services/db.dhall

let dbSync = ../services/cardano-dbsync.dhall

let cardanoNode = ../services/cardano-node.dhall

let cardanoWallet = ../services/cardano-wallet.dhall

let prismNode = ../services/prism-node.dhall

let scalaDid = ../services/scala-did.dhall

let ryo = ../services/ryo.dhall

let Options =
      { Type = { ci : Bool, ryo : Bool }
      , default = { ci = False, ryo = False }
      }

let mkStack =
      \(options : Options.Type) ->
        let networkMagic = 42

        let testnetVolume = "node-testnet"

        let cardanoNodeHost = "cardano-node"

        let walletId = "9263a1248b046fe9e1aabc4134b03dc5c3a7ee3d"

        let walletPassphrase = "super_secret"

        let walletPaymentAddress =
              "addr_test1qp83v2wq3z9mkcjj5ejlupgwt6tcly5mtmz36rpm8w4atvqd5jzpz23y8l4dwfd9l46fl2p86nmkkx5keewdevqxhlyslv99j3"

        in  { services =
              { cardano-node =
                  cardanoNode.mkNodeService
                    cardanoNode.NodeOptions::{ networkMagic, testnetVolume }
              , bootstrap-testnet =
                  cardanoNode.mkBootstrapService
                    cardanoNode.BootstrapOptions::{
                    , networkMagic
                    , testnetVolume
                    , cardanoNodeHost
                    , walletBaseUrl = "http://cardano-wallet:8090/v2"
                    , walletPassphrase
                    , walletPaymentAddress
                    , initWalletHurlFile = "./init-wallet.hurl"
                    , initDidHurlFile = "./init-did.hurl"
                    }
              , cardano-dbsync =
                  dbSync.mkService
                    dbSync.Options::{
                    , testnetVolume
                    , cardanoNodeHost
                    , dbHost = "db-dbsync"
                    , configFile = "./dbsync-config.yaml"
                    }
              , cardano-wallet =
                  cardanoWallet.mkService
                    cardanoWallet.Options::{
                    , testnetVolume
                    , cardanoNodeHost
                    , hostPort = Some 18081
                    }
              , bf-ryo =
                  let ryoService =
                        ryo.mkService
                          ryo.Options::{
                          , hostPort = Some 18082
                          , dbsyncDb = ryo.DbSyncDbArgs::{
                            , host = "db-dbsync"
                            , port = 5432
                            , dbName = "postgres"
                            , username = "postgres"
                            , password = "postgres"
                            }
                          , network = "custom"
                          , testnetVolume
                          , configFile = "./ryo.yaml"
                          , bootstrapTestnetHost = "bootstrap-testnet"
                          }

                  in  if    options.ryo
                      then  Some ryoService
                      else  None docker.Service.Type
              , neoprism-standalone =
                  neoprism.mkService
                    neoprism.Options::{
                    , imageOverride =
                        if    options.ci
                        then  Some "identus-neoprism:${neoprismVersion}"
                        else  None Text
                    , hostPort = Some 18080
                    , dbHost = "db-neoprism"
                    , confirmationBlocks = Some 0
                    , indexInterval = Some 1
                    , dltSource =
                        neoprism.DltSource.DbSync
                          neoprism.DbSyncDltSourceArgs::{
                          , url =
                              "postgresql://postgres:postgres@db-dbsync:5432/postgres"
                          , pollInterval = 1
                          }
                    , dltSink = Some neoprism.DltSink::{
                      , walletHost = "cardano-wallet"
                      , walletPort = 8090
                      , walletId
                      , walletPassphrase
                      , walletPaymentAddress
                      }
                    }
              , prism-node =
                  prismNode.mkService
                    prismNode.Options::{
                    , nodeDbHost = "db-prism-node"
                    , dbSyncDbHost = "db-dbsync"
                    , bootstrapTestnetHost = "bootstrap-testnet"
                    , walletApiHost = "cardano-wallet"
                    , walletPassphrase
                    , walletId
                    , walletPaymentAddress
                    , hostPort = Some 50053
                    , confirmationBlocks = 0
                    }
              , db-neoprism = db.mkService db.Options::{=}
              , db-dbsync = db.mkService db.Options::{=}
              , db-prism-node = db.mkService db.Options::{=}
              }
            , volumes = toMap { node-testnet = {=} }
            }

in  { mkStack, Options }
