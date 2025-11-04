# Simple OS 디버깅 스크립트 (Windows PowerShell)
# QEMU를 GDB 서버 모드로 실행합니다.

Write-Host "=== Simple OS 디버깅 모드 ===" -ForegroundColor Green
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

# QEMU를 GDB 서버 모드로 실행
Write-Host ""
Write-Host "[2/2] QEMU GDB 서버 시작 중..." -ForegroundColor Yellow
Write-Host "QEMU가 포트 1234에서 GDB 연결을 대기합니다." -ForegroundColor Cyan
Write-Host ""
Write-Host "다른 터미널에서 다음 명령을 실행하세요:" -ForegroundColor Yellow
Write-Host "  rust-gdb target\x86_64-unknown-none\debug\simple_os.exe" -ForegroundColor Cyan
Write-Host "  (gdb) target remote :1234" -ForegroundColor Cyan
Write-Host ""
Write-Host "종료하려면 Ctrl+C를 누르세요." -ForegroundColor Cyan
Write-Host ""

$bootimagePath = "target\x86_64-unknown-none\debug\bootimage-simple_os.bin"

if (-not (Test-Path $bootimagePath)) {
    Write-Host "오류: 부팅 이미지를 찾을 수 없습니다: $bootimagePath" -ForegroundColor Red
    exit 1
}

# QEMU를 GDB 서버 모드로 실행 (-s는 -gdb tcp::1234와 동일, -S는 시작 시 정지)
qemu-system-x86_64 `
    -s -S `
    -drive format=raw,file=$bootimagePath `
    -serial stdio `
    -display none

