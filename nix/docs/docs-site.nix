{
  stdenv,
  lib,
  version,
  d2,
  mdbook,
  mdbook-d2,
  mdbook-cmdrun,
  mdbook-linkcheck,
  yq-go,
  neoprism-bin,
}:

stdenv.mkDerivation {
  inherit version;
  pname = "docs-site";

  src = lib.cleanSource ../../docs;

  buildInputs = [
    d2
    mdbook
    mdbook-cmdrun
    mdbook-d2
    mdbook-linkcheck
    neoprism-bin
    yq-go
  ];

  buildPhase = ''
    mdbook build
  '';

  installPhase = ''
    mkdir -p $out
    cp -r book/* $out/
  '';
}
