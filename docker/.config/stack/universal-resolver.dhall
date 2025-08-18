let db = ../services/db.dhall

let neoprism = ../services/neoprism.dhall

let uniResolverWeb = ../services/uni-resolver-web.dhall

let mkStack =
      \(_ : {}) ->
        { services =
          { db = db.mkService db.Options::{ hostPort = Some 5432 }
          , neoprism-indexer =
              neoprism.mkService
                neoprism.Options::{
                , hostPort = Some 8081
                , dbHost = "db"
                , network = "mainnet"
                , dltSource =
                    neoprism.DltSource.Relay
                      "backbone.mainnet.cardanofoundation.org:3001"
                }
          , uni-resolver-web =
              uniResolverWeb.mkService uniResolverWeb.Options::{ port = 8080 }
          }
        }

in  { mkStack }
