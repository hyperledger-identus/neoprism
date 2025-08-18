let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let Options =
      { Type = { image : Text, port : Natural, prismDriverUrl : Text }
      , default =
        { image = "universalresolver/uni-resolver-web:0.43.0-362c5ca"
        , port = 8080
        , prismDriverUrl = "http://neoprism-indexer:8080"
        }
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image = options.image
        , ports = Some [ "${Prelude.Natural.show options.port}:8080" ]
        , environment = Some
            ( toMap
                { uniresolver_web_driver_url_did_prism = options.prismDriverUrl
                }
            )
        }

in  { Options, mkService }
