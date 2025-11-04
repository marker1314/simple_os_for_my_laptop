#!/bin/bash
# Simple OS QEMU 실행 스크립트 (Linux/macOS)
# 커널을 빌드하고 QEMU에서 실행합니다.

set -e

echo "=== Simple OS 빌드 및 실행 ==="
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

# QEMU 실행
echo ""
echo "[2/2] QEMU에서 커널 실행 중..."
echo "종료하려면 Ctrl+C를 누르세요."
echo ""

BOOTIMAGE_PATH="target/x86_64-unknown-none/debug/bootimage-simple_os.bin"

if [ ! -f "$BOOTIMAGE_PATH" ]; then
    echo "오류: 부팅 이미지를 찾을 수 없습니다: $BOOTIMAGE_PATH"
    exit 1
fi

# QEMU 실행 (시리얼 포트를 콘솔로 리다이렉트)
qemu-system-x86_64 \
    -drive format=raw,file="$BOOTIMAGE_PATH" \
    -serial stdio \
    -display none

