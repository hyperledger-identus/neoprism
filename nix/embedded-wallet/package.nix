{
  stdenv,
  bun,
  typescript,
  biome,
  version,
  targetBun ? null,
  bunTarget ? null,
  pnameSuffix ? null,
}:

let
  dependencyInputsHash = builtins.hashString "sha256" (
    builtins.readFile ../../packages/embedded-wallet/package.json
    + builtins.readFile ../../packages/embedded-wallet/bun.lock
    + bun.version
    + stdenv.buildPlatform.system
  );

  # Bun materializes platform-specific node_modules trees, so keep each hash
  # aligned with `bun install --frozen-lockfile` on its build platform. The
  # derivation name changes with declared inputs and the build platform so
  # normal CI builds cannot reuse stale output.
  # Build `.#embedded-wallet.deps` once, then audit same-input reproducibility
  # with `nix build .#embedded-wallet.deps --rebuild` (see issue #326).
  bunDepsHash =
    if stdenv.buildPlatform.isDarwin then
      "sha256-+fXtrsiupnGq0d4c5nt6R5qf/EJBxbv0PEkldCnZFi8="
    else if stdenv.buildPlatform.isLinux then
      "sha256-NUZE53F3BT5nXajXKZBCaiO3zccwY7uHhGyg/rxq1HQ="
    else
      throw "unsupported embedded-wallet dependency build platform: ${stdenv.buildPlatform.system}";

  bunDeps = stdenv.mkDerivation {
    name = "embedded-wallet-deps-${dependencyInputsHash}";
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

  passthru.deps = bunDeps;

  src = ./../..;

  nativeBuildInputs = [
    bun
    typescript
    biome
  ];

  # Disable stripping - it breaks bun-compiled executables by removing embedded bytecode
  dontStrip = true;

  buildPhase = ''
    export HOME=$TMPDIR

    # Copy dependencies to the expected location
    cd packages/embedded-wallet
    mkdir -p node_modules
    cp -r ${bunDeps}/. node_modules/

    # Verify source code
    biome format src/
    tsc --noEmit
    bun test

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
