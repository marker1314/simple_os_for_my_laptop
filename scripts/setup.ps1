# Simple OS 개발 환경 설정 스크립트 (Windows PowerShell)
# 이 스크립트는 Rust OS 개발에 필요한 모든 도구를 설치합니다.

Write-Host "=== Simple OS 개발 환경 설정 ===" -ForegroundColor Green
Write-Host ""

# Rust nightly 설치 확인 및 설치
Write-Host "[1/6] Rust nightly 툴체인 확인 중..." -ForegroundColor Yellow
$nightlyInstalled = rustup toolchain list | Select-String "nightly"
if (-not $nightlyInstalled) {
    Write-Host "Rust nightly가 설치되어 있지 않습니다. 설치를 시작합니다..." -ForegroundColor Yellow
    rustup install nightly
    rustup default nightly
    Write-Host "Rust nightly 설치 완료!" -ForegroundColor Green
} else {
    Write-Host "Rust nightly가 이미 설치되어 있습니다." -ForegroundColor Green
}

# 필수 컴포넌트 추가
Write-Host ""
Write-Host "[2/6] 필수 Rust 컴포넌트 추가 중..." -ForegroundColor Yellow
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
Write-Host "컴포넌트 추가 완료!" -ForegroundColor Green

# x86_64-unknown-none 타겟 추가
Write-Host ""
Write-Host "[3/6] x86_64-unknown-none 타겟 추가 중..." -ForegroundColor Yellow
rustup target add x86_64-unknown-none --toolchain nightly
Write-Host "타겟 추가 완료!" -ForegroundColor Green

# bootimage 설치 확인
Write-Host ""
Write-Host "[4/6] bootimage 도구 확인 중..." -ForegroundColor Yellow
$bootimageInstalled = cargo --list | Select-String "bootimage"
if (-not $bootimageInstalled) {
    Write-Host "bootimage가 설치되어 있지 않습니다. 설치를 시작합니다..." -ForegroundColor Yellow
    Write-Host "이 작업은 몇 분이 걸릴 수 있습니다..." -ForegroundColor Yellow
    cargo install bootimage --version "^0.11.0"
    Write-Host "bootimage 설치 완료!" -ForegroundColor Green
} else {
    Write-Host "bootimage가 이미 설치되어 있습니다." -ForegroundColor Green
}

# QEMU 설치 확인
Write-Host ""
Write-Host "[5/6] QEMU 설치 확인 중..." -ForegroundColor Yellow
$qemuInstalled = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemuInstalled) {
    Write-Host "QEMU가 설치되어 있지 않습니다." -ForegroundColor Yellow
    Write-Host "QEMU를 설치하려면 다음 중 하나를 실행하세요:" -ForegroundColor Yellow
    Write-Host "  1. winget install SoftwareFreedomConservancy.QEMU" -ForegroundColor Cyan
    Write-Host "  2. 또는 https://www.qemu.org/download/#windows 에서 다운로드" -ForegroundColor Cyan
} else {
    Write-Host "QEMU가 설치되어 있습니다: $($qemuInstalled.Source)" -ForegroundColor Green
}

# LLVM/clang 확인
Write-Host ""
Write-Host "[6/6] LLVM/clang 확인 중..." -ForegroundColor Yellow
$clangInstalled = Get-Command clang -ErrorAction SilentlyContinue
if (-not $clangInstalled) {
    Write-Host "clang이 설치되어 있지 않습니다." -ForegroundColor Yellow
    Write-Host "LLVM을 설치하려면:" -ForegroundColor Yellow
    Write-Host "  winget install LLVM.LLVM" -ForegroundColor Cyan
    Write-Host "또는 Visual Studio Build Tools를 설치하세요." -ForegroundColor Cyan
} else {
    Write-Host "clang이 설치되어 있습니다: $($clangInstalled.Source)" -ForegroundColor Green
}

# 최종 검증
Write-Host ""
Write-Host "=== 설정 완료 ===" -ForegroundColor Green
Write-Host ""
Write-Host "설치된 도구 확인:" -ForegroundColor Yellow
rustup show
Write-Host ""
Write-Host "설치된 타겟 확인:" -ForegroundColor Yellow
rustup target list --installed
Write-Host ""
Write-Host "다음 단계:" -ForegroundColor Green
Write-Host "  1. 'cargo build' 를 실행하여 프로젝트가 빌드되는지 확인하세요." -ForegroundColor Cyan
Write-Host "  2. '.\scripts\run.ps1' 를 실행하여 QEMU에서 커널을 테스트하세요." -ForegroundColor Cyan

