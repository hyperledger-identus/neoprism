{
  stdenv,
  fetchurl,
  unzip,
  autoPatchelfHook,
  glibc,
  gcc-unwrapped,
  util-linux,
}:

stdenv.mkDerivation rec {
  pname = "midnight-compactc";
  version = "0.24.0";
  src = fetchurl {
    url = "https://d3fazakqrumx6p.cloudfront.net/artifacts/compiler/compactc_${version}/compactc_v${version}_x86_64-unknown-linux-musl.zip";
    hash = "sha256-4pi2/kVe0DjUp2SWm6fUMNyylLDZtTJ8lKsaAjouSn0=";
  };
  nativeBuildInputs = [
    unzip
    autoPatchelfHook
  ];
  buildInputs = [
    glibc
    gcc-unwrapped
    util-linux
  ];
  unpackPhase = "true";
  installPhase = ''
    mkdir -p $out
    unzip $src -d $out/bin
  '';
  autoPatchelf = true;
}
