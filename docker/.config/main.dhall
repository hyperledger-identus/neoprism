let Prelude = (./prelude.dhall).Prelude

let neoprism = ./services/neoprism.dhall

let db = ./services/db.dhall

let uniResolver = ./stack/universal-resolver.dhall

let prismTest = ./stack/prism-test.dhall

in  { mainnet-dbsync.services
      =
      { db = db.mkService db.Options::{ hostPort = Some 5432 }
      , neoprism-indexer =
          neoprism.mkService
            neoprism.Options::{
            , hostPort = Some 8080
            , dltSource =
                neoprism.DltSource.DbSync
                  neoprism.DbSyncDltSourceArgs::{ url = "<DBSYNC_URL>" }
            }
      }
    , mainnet-relay.services
      =
      { db = db.mkService db.Options::{ hostPort = Some 5432 }
      , neoprism-indexer =
          neoprism.mkService
            neoprism.Options::{
            , hostPort = Some 8080
            , dltSource =
                neoprism.DltSource.Relay
                  "backbone.mainnet.cardanofoundation.org:3001"
            }
      }
    , preprod-relay.services
      =
      { db = db.mkService db.Options::{ hostPort = Some 5432 }
      , neoprism-indexer =
          neoprism.mkService
            neoprism.Options::{
            , hostPort = Some 8080
            , network = "preprod"
            , dltSource =
                neoprism.DltSource.Relay
                  "preprod-node.play.dev.cardano.org:3001"
            }
      }
    , prism-test = prismTest.mkStack prismTest.Options::{=}
    , prism-test-ci = prismTest.mkStack prismTest.Options::{ ci = True }
    , prism-test-ryo = prismTest.mkStack prismTest.Options::{ ryo = True }
    , mainnet-universal-resolver = uniResolver.mkStack {=}
    }
