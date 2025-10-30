{
  lib,
  mkSbtDerivation,
  writeShellApplication,
  fetchFromGitHub,
  stdenv,
  cacert,
  protobuf,
  temurin-jre-bin,
  symlinkJoin,
}:

let
  jars = mkSbtDerivation rec {
    pname = "scala-did";
    version = "v0.1.0-M28";
    depsSha256 = "sha256-E5APuu4m5U3dyfvvBi/xrIgQBwmhGoD1CgYzyXp4tuw=";
    src = fetchFromGitHub {
      owner = "FabioPinheiro";
      repo = "scala-did";
      rev = version;
      hash = "sha256-NMeiJrxOsa0F9oJqNm85DGtfsstunwWwJUrDZImdJ8w=";
    };
    patches = [ ./node_impl.patch ];

    nativeBuildInputs = [
      cacert
      protobuf
    ];

    LANG = "C.utf8";
    SSL_CERT_FILE = "${cacert}/etc/ssl/certs/ca-bundle.crt";

    depsWarmupCommand = ''
      export LD_LIBRARY_PATH=${
        lib.makeLibraryPath [
          stdenv.cc.cc
        ]
      }
      sbt didResolverPrismJVM/compile
      sbt didPrismNode/compile
    '';

    buildPhase = ''
      export LD_LIBRARY_PATH=${
        lib.makeLibraryPath [
          stdenv.cc.cc
        ]
      }
      sbt didResolverPrismJVM/assembly
      sbt didPrismNode/assembly
    '';

    installPhase = ''
      mkdir -p $out/jars
      cp -r did-method-prism/jvm/target/scala-3.3.6/cardano-prism.jar $out/jars/prism-cli.jar
      cp -r did-method-prism-node/target/scala-3.3.6/prism-node.jar $out/jars/prism-node.jar
    '';
  };
  prism-cli = writeShellApplication {
    name = "prism-cli";
    runtimeInputs = [ temurin-jre-bin ];
    text = ''
      if [ -z "''${PRISM_HOME:-}" ]; then
        echo "Error: PRISM_HOME cannot be empty."
        exit 1
      fi
      java -Duser.home="$PRISM_HOME" -jar ${jars}/jars/prism-cli.jar "$@"
    '';
  };

  scala-did-node = writeShellApplication {
    name = "scala-did-node";
    runtimeInputs = [ temurin-jre-bin ];
    text = ''
      java -jar ${jars}/jars/prism-node.jar "$@"
    '';
  };
in
symlinkJoin {
  name = "scala-did";
  paths = [
    prism-cli
    scala-did-node
    jars
  ];
}
