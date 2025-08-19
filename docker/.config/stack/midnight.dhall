let docker = ../docker.dhall

let db = ../services/db.dhall

let mkStack =
      \(_ : {}) ->
        { services =
          { db = db.mkService db.Options::{=}
          , midnight-proof-server = docker.Service::{
            , image = "midnightnetwork/proof-server:4.0.0"
            , ports = Some [ "6300:6300" ]
            , command = Some [ "midnight-proof-server", "--network", "testnet" ]
            }
          }
        }

in  { mkStack }
