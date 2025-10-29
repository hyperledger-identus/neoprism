let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "ghcr.io/intersectmbo/cardano-submit-api:10.4.1"

let Options =
      { Type =
          { hostPort : Optional Natural
          , testnetVolume : Text
          , cardanoNodeHost : Text
          , networkMagic : Natural
          }
      , default.hostPort = None Natural
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image
        , ports =
            Prelude.Optional.map
              Natural
              (List Text)
              (\(n : Natural) -> [ "${Prelude.Natural.show n}:8090" ])
              options.hostPort
        , command = Some
          [ "--config"
          , "/config.yaml"
          , "--socket-path"
          , "/node/testnet/socket/node1/sock"
          , "--testnet-magic"
          , Prelude.Natural.show options.networkMagic
          , "--listen-address"
          , "0.0.0.0"
          ]
        , volumes = Some
          [ "${options.testnetVolume}:/node/testnet"
          , "./cardano-submit-api.yaml:/config.yaml"
          ]
        , depends_on = Some
          [ docker.ServiceCondition.healthy options.cardanoNodeHost ]
        }

in  { Options, mkService }
