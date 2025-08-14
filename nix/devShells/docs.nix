{ pkgs }:

pkgs.mkShell {
  name = "docs-shell";
  buildInputs = with pkgs; [
    mdbook
    # Add mdBook plugins here if needed, e.g.:
    # mdbook-mermaid
    # mdbook-toc
    # mdbook-linkcheck
  ];
  shellHook = ''
    echo "Welcome to the NeoPRISM documentation shell!"
    echo "Run 'mdbook serve' or 'mdbook build' in the docs/ directory."
  '';
}
