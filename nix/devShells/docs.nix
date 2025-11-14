{ self, pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  name = "docs-shell";
  buildInputs = with pkgs; [
    d2
    mdbook
    mdbook-cmdrun
    mdbook-d2
    mdbook-linkcheck
    yq-go
    self.packages.${stdenv.hostPlatform.system}.neoprism-bin
  ];
  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}/docs"
  '';
}
