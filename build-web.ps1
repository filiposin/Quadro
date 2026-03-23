rustup target add wasm32-unknown-unknown

if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    cargo install wasm-pack
}

wasm-pack build --release --target web --out-dir docs/pkg

New-Item -ItemType File -Path "docs/.nojekyll" -Force | Out-Null

Write-Host ""
Write-Host "Сборка готова."
Write-Host "Для локального просмотра можно выполнить:"
Write-Host "python -m http.server 8080 --directory docs"
Write-Host "и открыть http://localhost:8080"
