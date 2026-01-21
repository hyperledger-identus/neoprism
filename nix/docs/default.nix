{ ... }:

{
  perSystem =
    {
      pkgs,
      self',
      neoprismLib,
      ...
    }:
    let
      inherit (neoprismLib) version;
    in
    {
      packages = {
        docs-site = pkgs.callPackage ./docs-site.nix {
          inherit version;
          neoprism-bin = self'.packages.neoprism-bin;
        };
      };

      devShells.docs = pkgs.mkShell {
        name = "docs-shell";
        buildInputs = with pkgs; [
          d2
          mdbook
          mdbook-cmdrun
          mdbook-d2
          mdbook-linkcheck
          yq-go
          self'.packages.neoprism-bin
        ];
        shellHook = ''
          export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
          ${pkgs.cowsay}/bin/cowsay "Working on project root directory: $ROOT_DIR"
          cd "$ROOT_DIR/docs"
        '';
      };
    };
}
