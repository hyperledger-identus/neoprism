{
  bash,
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
  extraCommands = ''
    install -d -m 700 var/lib/neoprism/sqlite
    touch var/lib/neoprism/sqlite/.keep
  '';
  contents = [
    bash
    curl
    neoprism-bin
    neoprism-ui-assets
  ]
  ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
      "NPRISM_ASSETS_PATH=/assets"
    ];
    Entrypoint = [ "/bin/neoprism-node" ];
    Cmd = [ ];
    WorkingDir = "/";
    Volumes = {
      "/var/lib/neoprism/sqlite" = { };
    };
  };
}
