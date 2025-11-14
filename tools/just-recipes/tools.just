set working-directory := '../..'

# Format and lint-fix tools sources and justfile
format:
    echo "Formatting Python files..."
    ruff check --select I --fix tools/compose_gen
    ruff format tools/compose_gen

    echo "Formatting justfile..."
    just --fmt --unstable

# Type check and validate Python tools code
check:
    #!/usr/bin/env bash
    SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    nix build ".#checks.$SYSTEM.tools"
