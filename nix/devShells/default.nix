{ self, pkgs }:

let
  rootDir = "$ROOT_DIR";
  buildConfig = pkgs.writeShellApplication {
    name = "buildConfig";
    runtimeInputs = with pkgs; [ dhall-json ];
    text = ''
      cd "${rootDir}/docker/.config"
      dhall-to-yaml <<< "(./main.dhall).mainnet-dbsync" > "${rootDir}/docker/mainnet-dbsync/compose.yml"
      dhall-to-yaml <<< "(./main.dhall).mainnet-relay" > "${rootDir}/docker/mainnet-relay/compose.yml"
      dhall-to-yaml <<< "(./main.dhall).preprod-relay" > "${rootDir}/docker/preprod-relay/compose.yml"
      dhall-to-yaml <<< "(./main.dhall).prism-test" > "${rootDir}/docker/prism-test/compose.yml"
      dhall-to-yaml <<< "(./main.dhall).prism-test-ci" > "${rootDir}/docker/prism-test/compose-ci.yml"
      dhall-to-yaml <<< "(./main.dhall).mainnet-universal-resolver" > "${rootDir}/docker/mainnet-universal-resolver/compose.yml"
      dhall-to-yaml <<< "(./main.dhall).midnight" > "${rootDir}/docker/midnight/compose.yml"
    '';
  };
in
{
  default = import ./neoprism.nix { inherit pkgs buildConfig; };
  release = import ./release.nix { inherit pkgs buildConfig; };
  cardano = import ./cardano.nix { inherit pkgs; };
  midnight = import ./midnight.nix { inherit pkgs; };
  prism-test = import ./prism-test.nix { inherit pkgs; };
  docs = import ./docs.nix { inherit pkgs self; };
}
