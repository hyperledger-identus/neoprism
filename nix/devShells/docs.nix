{ self, pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  name = "docs-shell";
  buildInputs = with pkgs; [
    mdbook
    mdbook-cmdrun
    self.packages.${pkgs.system}.neoprism-bin
  ];
  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}/docs"
  '';
}
