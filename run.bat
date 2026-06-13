@echo off
title MLBV - Minecraft Launcher by vlal
cd /d "%~dp0"

echo.
echo  MLBV - Dev Build
echo  ----------------
echo  First launch compiles Rust - takes 5-15 min
echo  Subsequent launches: ~20 sec
echo.

where node >nul 2>&1
if errorlevel 1 (
    echo  ERROR: Node.js not found in PATH
    echo  Install from: https://nodejs.org
    pause
    exit /b 1
)

where cargo >nul 2>&1
if errorlevel 1 (
    echo  ERROR: Rust/Cargo not found in PATH
    echo  Install from: https://rustup.rs
    pause
    exit /b 1
)

npm run tauri dev
if errorlevel 1 (
    echo.
    echo  Build failed - see error above
    pause
)
