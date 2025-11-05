<#
 Simple OS Bootimage Builder (PowerShell)
 Canonical method: use cargo-bootimage (bootimage crate) to build and create a bootable image.
#>

param(
    [switch]$Release
)

Write-Host "=== Building Simple OS Bootimage ===" -ForegroundColor Green

if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Error: Cargo.toml not found. Run from project root." -ForegroundColor Red
    exit 1
}

$toolchain = "+nightly"
$profile = if ($Release) { "--release" } else { "" }

Write-Host "[1/2] Installing/checking bootimage subcommand..." -ForegroundColor Yellow
cargo install bootimage | Out-Null

Write-Host "[2/2] Building bootimage..." -ForegroundColor Yellow
cargo $toolchain bootimage $profile
if ($LASTEXITCODE -ne 0) {
    Write-Host "Bootimage build failed." -ForegroundColor Red
    exit 1
}

$mode = if ($Release) { "release" } else { "debug" }
$bootimagePath = Join-Path "target/x86_64-unknown-none/$mode" "bootimage-simple_os.bin"

if (Test-Path $bootimagePath) {
    Write-Host "Bootimage ready: $bootimagePath" -ForegroundColor Green
} else {
    Write-Host "Warning: bootimage not found at expected path: $bootimagePath" -ForegroundColor Yellow
}

