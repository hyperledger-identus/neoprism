{
  curl,
  dockerTools,
  neoprism-bin,
  tagSuffix ? "",
  neoprism-ui-assets,
  version,
  extraPackages ? [ ],
}:

dockerTools.buildLayeredImage {
  name = "identus-neoprism";
  tag = "${version}${tagSuffix}";
  contents = [
    curl
    neoprism-bin
    neoprism-ui-assets
  ] ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
      "NPRISM_ASSETS_PATH=/assets"
    ];
    Entrypoint = [ "/bin/neoprism-node" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
