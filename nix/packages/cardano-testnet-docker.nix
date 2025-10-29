{
  bash,
  cardano-cli,
  cardano-node,
  cardano-testnet,
  coreutils,
  dockerTools,
  gawk,
  gnugrep,
  hurl,
  jq,
  tagSuffix ? "",
  writeShellApplication,
}:

let
  scripts = {
    initTestnet = writeShellApplication {
      name = "initTestnet";
      text = ''
        cardano-testnet cardano \
          --conway-era \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --active-slots-coeff 0.1 \
          --epoch-length 60 \
          --output-dir testnet
      '';
    };
    transactGenesis = writeShellApplication {
      name = "transactGenesis";
      text = ''
        mkdir -p transactions

        echo "Getting utxos"
        TX_IN="$(cardano-cli conway query utxo --address "$(cat ./testnet/utxo-keys/utxo1/utxo.addr)" | jq -r '. | keys[]')"

        echo "Drafting transaction"
        cardano-cli conway transaction build \
          --tx-in "$TX_IN" \
          --tx-out "$GENESIS_PAYMENT_ADDR+100000000000" \
          --change-address "$GENESIS_PAYMENT_ADDR" \
          --out-file transactions/tx.raw

        echo "Signing transaction"
        cardano-cli conway transaction sign \
          --tx-body-file transactions/tx.raw \
          --signing-key-file ./testnet/utxo-keys/utxo1/utxo.skey \
          --out-file transactions/tx.signed

        echo "Submitting transaction"
        cardano-cli conway transaction submit --tx-file transactions/tx.signed
      '';
    };
  };
  basePackages = [
    bash
    coreutils
    gawk
    gnugrep
    hurl
    jq
  ];
  cardanoPackages = [
    cardano-cli
    cardano-node
    cardano-testnet
  ];
in
dockerTools.buildLayeredImage {
  name = "cardano-testnet";
  tag = "latest${tagSuffix}";
  contents = basePackages ++ cardanoPackages ++ (builtins.attrValues scripts);
  config = {
    Env = [
      "CARDANO_CLI=${cardano-cli}/bin/cardano-cli"
      "CARDANO_NODE=${cardano-node}/bin/cardano-node"
    ];
    Entrypoint = [ ];
    Cmd = [ ];
    WorkingDir = "/node";
  };
}
