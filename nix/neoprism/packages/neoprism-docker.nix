{
  bash,
  curl,
  dockerTools,
  neoprism-bin,
  embedded-wallet,
  tagSuffix ? "",
  neoprism-ui-assets,
  version,
  extraPackages ? [ ],
  openssl,
  cacert,
}:

dockerTools.buildLayeredImage {
  name = "identus-neoprism";
  tag = "${version}${tagSuffix}";
  contents = [
    bash
    cacert
    curl
    embedded-wallet
    neoprism-bin
    neoprism-ui-assets
    openssl
  ]
  ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
      "NPRISM_ASSETS_PATH=/assets"
      "NPRISM_EMBEDDED_WALLET_BIN=/bin/embedded-wallet"
      "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
    ];
    Entrypoint = [ "/bin/neoprism-node" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
