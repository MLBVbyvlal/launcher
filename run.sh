#!/usr/bin/env bash
# MLBV dev launcher for Linux/macOS (run.bat is the Windows equivalent).
cd "$(dirname "$0")"

echo ""
echo " MLBV - Dev Build"
echo " ----------------"
echo " First launch compiles Rust - takes 5-15 min"
echo " Subsequent launches: ~20 sec"
echo ""

if ! command -v node >/dev/null 2>&1; then
    echo " ERROR: Node.js not found in PATH"
    echo " Install from: https://nodejs.org"
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo " ERROR: Rust/Cargo not found in PATH"
    echo " Install from: https://rustup.rs"
    exit 1
fi

npm run tauri dev
