let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "caddy:2.10.2"

let Options =
      { Type =
          { imageOverride : Optional Text
          , hostPort : Optional Natural
          , targetPort : Natural
          , caddyfile : Text
          }
      , default =
        { imageOverride = None Text
        , hostPort = None Natural
        , targetPort = 3000
        , caddyfile = "./Caddyfile"
        }
      }

let mkService =
      \(options : Options.Type) ->
        docker.Service::{
        , image = Prelude.Optional.default Text image options.imageOverride
        , ports =
            Prelude.Optional.map
              Natural
              (List Text)
              ( \(p : Natural) ->
                  [ "${Prelude.Natural.show p}:${Prelude.Natural.show
                                                   options.targetPort}"
                  ]
              )
              options.hostPort
        , volumes = Some [ "${options.caddyfile}:/etc/caddy/Caddyfile" ]
        }

in  { mkService, Options }
