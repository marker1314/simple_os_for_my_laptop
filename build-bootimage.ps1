# Simple OS Bootimage Builder
# Builds the kernel and creates a bootable image using bootloader 0.11.x

Write-Host "=== Building Simple OS Kernel ===" -ForegroundColor Green

# Step 1: Build the kernel
Write-Host "[1/2] Building kernel..." -ForegroundColor Yellow
cargo +nightly build --target x86_64-unknown-none
if ($LASTEXITCODE -ne 0) {
    Write-Host "Kernel build failed!" -ForegroundColor Red
    exit 1
}

# Step 2: Create bootimage using bootloader crate's API
Write-Host "[2/2] Creating bootimage..." -ForegroundColor Yellow

$kernelPath = "target\x86_64-unknown-none\debug\simple_os"
$bootimagePath = "target\x86_64-unknown-none\debug\bootimage-simple_os.bin"

# Use cargo run with a temporary build script to create bootimage
# This is a workaround since bootimage 0.10.3 doesn't support bootloader 0.11.x
$buildScript = @"
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let kernel = PathBuf::from("$kernelPath");
    let output = PathBuf::from("$bootimagePath");
    
    // Use bootloader's build API
    bootloader::BootloaderBuilder::new()
        .kernel_binary_path(&kernel)
        .create_disk_image(&output)
        .expect("Failed to create bootimage");
}
"@

# Create temporary crate to build bootimage
$tempDir = "target\.bootimage_builder"
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
}
New-Item -ItemType Directory -Path $tempDir | Out-Null

$cargoToml = @"
[package]
name = "bootimage_builder"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "bootimage_builder"
path = "main.rs"

[dependencies]
bootloader = "0.11.12"
"@

Set-Content -Path "$tempDir\Cargo.toml" -Value $cargoToml
Set-Content -Path "$tempDir\main.rs" -Value $buildScript

Push-Location $tempDir
cargo +nightly run --release
$buildResult = $LASTEXITCODE
Pop-Location

if ($buildResult -ne 0) {
    Write-Host "Bootimage creation failed! Trying alternative method..." -ForegroundColor Yellow
    
    # Alternative: Use cargo-bootimage directly if available
    cargo-bootimage build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "All bootimage creation methods failed!" -ForegroundColor Red
        Write-Host "Kernel built successfully at: $kernelPath" -ForegroundColor Yellow
        Write-Host "You may need to create the bootimage manually or wait for bootimage 0.11.x" -ForegroundColor Yellow
        exit 1
    }
}

if (Test-Path $bootimagePath) {
    Write-Host "Bootimage created successfully: $bootimagePath" -ForegroundColor Green
} else {
    Write-Host "Warning: Bootimage file not found, but build completed." -ForegroundColor Yellow
    Write-Host "Kernel binary is at: $kernelPath" -ForegroundColor Yellow
}

