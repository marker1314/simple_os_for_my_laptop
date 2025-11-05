#!/bin/bash
# Simple OS 디버깅 스크립트 (Linux/macOS)
# QEMU를 GDB 서버 모드로 실행합니다.

set -e

echo "=== Simple OS 디버깅 모드 ==="
echo ""

# 현재 디렉토리가 프로젝트 루트인지 확인
if [ ! -f "Cargo.toml" ]; then
    echo "오류: Cargo.toml을 찾을 수 없습니다. 프로젝트 루트에서 실행하세요."
    exit 1
fi

# bootimage로 부팅 이미지 생성
echo "[1/2] 커널 빌드 중..."
cargo bootimage
if [ $? -ne 0 ]; then
    echo "빌드 실패!"
    exit 1
fi
echo "빌드 완료!"

# QEMU를 GDB 서버 모드로 실행
echo ""
echo "[2/2] QEMU GDB 서버 시작 중..."
echo "QEMU가 포트 1234에서 GDB 연결을 대기합니다."
echo ""
echo "다른 터미널에서 다음 명령을 실행하세요:"
echo "  rust-gdb target/x86_64-unknown-none/debug/simple_os"
echo "  (gdb) target remote :1234"
echo ""
echo "종료하려면 Ctrl+C를 누르세요."
echo ""

BOOTIMAGE_PATH="target/x86_64-unknown-none/debug/bootimage-simple_os.bin"

if [ ! -f "$BOOTIMAGE_PATH" ]; then
    echo "오류: 부팅 이미지를 찾을 수 없습니다: $BOOTIMAGE_PATH"
    exit 1
fi

# 부팅 타임라인 캡처를 위한 로그 파일
TIMELINE_LOG="boot_timeline_$(date +%Y%m%d_%H%M%S).log"

echo "부팅 타임라인 로그: $TIMELINE_LOG"
echo ""

# QEMU를 GDB 서버 모드로 실행 (-s는 -gdb tcp::1234와 동일, -S는 시작 시 정지)
# 시리얼 출력을 파일로도 저장
qemu-system-x86_64 \
    -s -S \
    -drive format=raw,file="$BOOTIMAGE_PATH" \
    -serial stdio 2>&1 | tee "$TIMELINE_LOG"

