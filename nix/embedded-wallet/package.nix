{
  stdenv,
  bun,
  version,
  targetBun ? null,
  bunTarget ? null,
  pnameSuffix ? null,
}:

let
  bunDepsHash = "sha256-NUZE53F3BT5nXajXKZBCaiO3zccwY7uHhGyg/rxq1HQ=";

  bunDeps = stdenv.mkDerivation {
    name = "embedded-wallet-deps";
    nativeBuildInputs = [ bun ];

    src = ./../..;

    outputHash = bunDepsHash;
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";

    installPhase = ''
      export HOME=$TMPDIR
      cd packages/embedded-wallet
      bun install --frozen-lockfile
      mkdir -p $out
      cp -r node_modules/. $out/
    '';
  };

  # Determine the target flag for bun build
  targetFlag = if bunTarget != null then "--target=${bunTarget}" else "--target=bun";

  # Determine whether to use compile-executable-path with nixpkgs bun
  executablePathFlag =
    if targetBun != null then "--compile-executable-path=${targetBun}/bin/bun" else "";

  pname = if pnameSuffix != null then "embedded-wallet-${pnameSuffix}" else "embedded-wallet";
in
stdenv.mkDerivation {
  inherit pname version;

  src = ./../..;

  nativeBuildInputs = [ bun ];

  # Disable stripping - it breaks bun-compiled executables by removing embedded bytecode
  dontStrip = true;

  buildPhase = ''
    export HOME=$TMPDIR

    # Copy dependencies to the expected location
    cd packages/embedded-wallet
    mkdir -p node_modules
    cp -r ${bunDeps}/. node_modules/

    # Build the binary
    # For cross-compilation: uses --target and --compile-executable-path with nixpkgs bun
    # For native: uses --target=bun which embeds the current platform's bun runtime
    bun build ./src/cli.ts --compile ${targetFlag} ${executablePathFlag} --outfile embedded-wallet
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp embedded-wallet $out/bin/embedded-wallet
    chmod +x $out/bin/embedded-wallet
  '';
}
