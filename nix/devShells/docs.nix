{ pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  name = "docs-shell";
  buildInputs = with pkgs; [
    mdbook
  ];
  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}/docs"
  '';
}
