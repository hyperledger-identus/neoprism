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
in
stdenv.mkDerivation {
  name = "tools-checks";
  src = lib.cleanSourceWith {
    filter =
      path: _:
      let
        baseName = builtins.baseNameOf path;
        relativePath = lib.removePrefix (toString ./../..) (toString path);
      in
      baseName == "justfile" || lib.hasPrefix "/tools" relativePath;
    src = ./../..;
  };

  nativeBuildInputs = [
    pythonEnv
    ruff
    pyright
    just
  ];

  buildPhase = "true";

  doCheck = true;

  checkPhase = ''
    echo "Checking justfile formatting..."
    just --unstable --check --fmt
    find . -name '*.just' -type f -print0 | xargs -0 -I {} sh -c 'echo "  → {}" && just --unstable --check --fmt --justfile {}'

    cd tools

    echo "Linting Python files..."
    ruff check compose_gen

    echo "Type checking Python files..."
    pyright compose_gen
  '';

  installPhase = "touch $out";
}
