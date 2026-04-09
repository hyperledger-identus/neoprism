set working-directory := '../..'

embedded_wallet_dir := 'packages/embedded-wallet'

install:
    cd {{ embedded_wallet_dir }} && bun install

typecheck:
    cd {{ embedded_wallet_dir }} && bun run typecheck

test:
    cd {{ embedded_wallet_dir }} && bun test

build:
    cd {{ embedded_wallet_dir }} && bun run build

format:
    cd {{ embedded_wallet_dir }} && biome format --write src/

check: typecheck test
    cd {{ embedded_wallet_dir }} && biome format src/
    echo "✓ Embedded wallet checks completed successfully."
