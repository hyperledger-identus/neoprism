let Prelude = (../prelude.dhall).Prelude

let docker = ../docker.dhall

let image = "patextreme/cardano-testnet:20250730-211809"

let NodeOptions =
      { Type = { networkMagic : Natural, testnetVolume : Text }, default = {=} }

let mkNodeService =
      \(options : NodeOptions.Type) ->
        docker.Service::{
        , image
        , restart = None Text
        , command = Some [ "initTestnet" ]
        , volumes = Some [ "${options.testnetVolume}:/node/testnet" ]
        , environment = Some
            ( toMap
                { CARDANO_NODE_SOCKET_PATH = "/node/testnet/socket/node1/sock"
                , CARDANO_NODE_NETWORK_ID =
                    Prelude.Natural.show options.networkMagic
                }
            )
        , healthcheck = Some docker.Healthcheck::{
          , test = [ "CMD-SHELL", "cardano-cli query tip" ]
          }
        }

let BootstrapOptions =
      { Type =
          { networkMagic : Natural
          , testnetVolume : Text
          , cardanoNodeHost : Text
          , walletBaseUrl : Text
          , walletPassphrase : Text
          , walletPaymentAddress : Text
          , initWalletHurlFile : Text
          , initDidHurlFile : Text
          }
      , default = {=}
      }

let mkBootstrapService =
      \(options : BootstrapOptions.Type) ->
        docker.Service::{
        , image
        , restart = None Text
        , volumes = Some
          [ "${options.testnetVolume}:/node/testnet"
          , "${options.initWalletHurlFile}:/node/init-wallet.hurl"
          , "${options.initDidHurlFile}:/node/init-did.hurl"
          ]
        , command = Some
          [ "bash"
          , "-c"
          , ''
            transactGenesis
            hurl ./init-wallet.hurl
            hurl ./init-did.hurl

            # blockfrost-ryo expects a different location
            cp testnet/conway-genesis.json testnet/genesis.json
            cp testnet/byron-genesis.json testnet/byron_genesis.json
            ''
          ]
        , environment = Some
            ( toMap
                { HURL_WALLET_BASE_URL = options.walletBaseUrl
                , HURL_WALLET_PASSPHRASE = options.walletPassphrase
                , GENESIS_PAYMENT_ADDR = options.walletPaymentAddress
                , CARDANO_NODE_SOCKET_PATH = "/node/testnet/socket/node1/sock"
                , CARDANO_NODE_NETWORK_ID =
                    Prelude.Natural.show options.networkMagic
                }
            )
        , depends_on = Some
          [ docker.ServiceCondition.healthy options.cardanoNodeHost ]
        }

in  { NodeOptions, mkNodeService, BootstrapOptions, mkBootstrapService }
