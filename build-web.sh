#!/usr/bin/env bash
set -euo pipefail

rustup target add wasm32-unknown-unknown

if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack
fi

wasm-pack build --release --target web --out-dir docs/pkg

touch docs/.nojekyll

echo
echo 'Сборка готова.'
echo 'Для локального просмотра выполни:'
echo 'python3 -m http.server 8080 --directory docs'
echo 'и открой http://localhost:8080'
