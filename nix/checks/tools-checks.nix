{
  lib,
  stdenv,
  pythonTools,
  ruff,
  pyright,
  just,
}:

let
  inherit (pythonTools) pythonEnv;
  justfile = ./../../justfile;
in
stdenv.mkDerivation {
  name = "tools-checks";
  src = lib.cleanSource ./../../tools;

  nativeBuildInputs = [
    pythonEnv
    ruff
    pyright
    just
  ];

  buildPhase = "true";

  doCheck = true;

  checkPhase = ''
    echo "Linting Python files..."
    ruff check compose_gen

    echo "Type checking Python files..."
    pyright compose_gen

    echo "Checking justfile formatting..."
    cp ${justfile} justfile
    just --fmt --unstable --check justfile
  '';

  installPhase = "touch $out";
}
