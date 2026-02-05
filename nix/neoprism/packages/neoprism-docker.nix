{
  bash,
  curl,
  dockerTools,
  neoprism-bin,
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
    neoprism-bin
    neoprism-ui-assets
    openssl
  ]
  ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
      "NPRISM_ASSETS_PATH=/assets"
      "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
    ];
    Entrypoint = [ "/bin/neoprism-node" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
