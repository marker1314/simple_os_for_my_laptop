# Simple OS QEMU 실행 스크립트 (Windows PowerShell)
# 커널을 빌드하고 QEMU에서 실행합니다.

Write-Host "=== Simple OS 빌드 및 실행 ===" -ForegroundColor Green
Write-Host ""

# 현재 디렉토리가 프로젝트 루트인지 확인
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "오류: Cargo.toml을 찾을 수 없습니다. 프로젝트 루트에서 실행하세요." -ForegroundColor Red
    exit 1
}

# bootimage로 부팅 이미지 생성
Write-Host "[1/2] 커널 빌드 중..." -ForegroundColor Yellow
cargo bootimage
if ($LASTEXITCODE -ne 0) {
    Write-Host "빌드 실패!" -ForegroundColor Red
    exit 1
}
Write-Host "빌드 완료!" -ForegroundColor Green

# QEMU 실행
Write-Host ""
Write-Host "[2/2] QEMU에서 커널 실행 중..." -ForegroundColor Yellow
Write-Host "종료하려면 Ctrl+C를 누르세요." -ForegroundColor Cyan
Write-Host ""

$bootimagePath = "target\x86_64-unknown-none\debug\bootimage-simple_os.bin"

if (-not (Test-Path $bootimagePath)) {
    Write-Host "오류: 부팅 이미지를 찾을 수 없습니다: $bootimagePath" -ForegroundColor Red
    exit 1
}

# QEMU 실행 (시리얼 포트를 콘솔로 리다이렉트)
qemu-system-x86_64 `
    -drive format=raw,file=$bootimagePath `
    -serial stdio `
    -display none

