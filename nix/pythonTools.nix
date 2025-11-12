{ python313 }:

let
  commonPackages =
    ps: with ps; [
      pydantic
      pyyaml
    ];
in
rec {
  python = python313;

  pythonEnv = python.withPackages commonPackages;

  mkPythonEnv =
    {
      packages ? commonPackages,
    }:
    python.withPackages packages;
}
