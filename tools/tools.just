# Format and lint-fix tools sources and justfile
format:
    echo "Formatting Python files..."
    ruff check --select I --fix compose_gen
    ruff format compose_gen

    echo "Formatting justfile..."
    just --fmt --unstable

# Type check and validate Python tools code
[working-directory('..')]
check:
    #!/usr/bin/env bash
    SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
    nix build ".#checks.$SYSTEM.tools"
