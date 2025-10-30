let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let Options =
      { Type = { image : Text, hostPort : Natural, prismDriverUrl : Text }
      , default =
        { image = "universalresolver/uni-resolver-web:0.44.0-4922fcc"
        , prismDriverUrl = "http://neoprism-indexer:8080/api"
        }
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image = options.image
        , ports = Some [ "${Prelude.Natural.show options.hostPort}:8080" ]
        , environment = Some
            ( toMap
                { uniresolver_web_driver_url_did_prism = options.prismDriverUrl
                }
            )
        }

in  { Options, mkService }
