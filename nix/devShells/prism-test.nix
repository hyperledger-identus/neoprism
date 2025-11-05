{ pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  packages = with pkgs; [
    git
    jdk
    metals
    ncurses
    sbt
  ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}/tests/prism-test"
  '';

  JAVA_HOME = "${pkgs.jdk}/lib/openjdk";
  SBT_OPTS = "-Xmx4G";
  SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

  LANG = "C.utf8";
  LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib/"; # required by scalapb
}
