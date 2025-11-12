{
  lib,
  stdenv,
  pythonTools,
  ruff,
  pyright,
}:

let
  inherit (pythonTools) pythonEnv;
in
stdenv.mkDerivation {
  name = "python-tools-checks";
  src = lib.cleanSource ./../../tools;

  nativeBuildInputs = [
    pythonEnv
    ruff
    pyright
  ];

  buildPhase = "true";

  checkPhase = ''
    echo "Linting Python files..."
    ruff check compose_gen

    echo "Type checking Python files..."
    pyright compose_gen
  '';

  installPhase = "touch $out";
}
