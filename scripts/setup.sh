#!/bin/bash
# Simple OS 개발 환경 설정 스크립트 (Linux/macOS)
# 이 스크립트는 Rust OS 개발에 필요한 모든 도구를 설치합니다.

set -e

echo "=== Simple OS 개발 환경 설정 ==="
echo ""

# Rust nightly 설치 확인 및 설치
echo "[1/7] Rust nightly 툴체인 확인 중..."
if ! rustup toolchain list | grep -q "nightly"; then
    echo "Rust nightly가 설치되어 있지 않습니다. 설치를 시작합니다..."
    rustup install nightly
    rustup default nightly
    echo "Rust nightly 설치 완료!"
else
    echo "Rust nightly가 이미 설치되어 있습니다."
fi

# 필수 컴포넌트 추가
echo ""
echo "[2/7] 필수 Rust 컴포넌트 추가 중..."
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
echo "컴포넌트 추가 완료!"

# x86_64-unknown-none 타겟 추가
echo ""
echo "[3/7] x86_64-unknown-none 타겟 추가 중..."
rustup target add x86_64-unknown-none --toolchain nightly
echo "타겟 추가 완료!"

# bootimage 설치 확인
echo ""
echo "[4/7] bootimage 도구 확인 중..."
if ! cargo --list | grep -q "bootimage"; then
    echo "bootimage가 설치되어 있지 않습니다. 설치를 시작합니다..."
    echo "이 작업은 몇 분이 걸릴 수 있습니다..."
    cargo install bootimage --version "^0.11.0"
    echo "bootimage 설치 완료!"
else
    echo "bootimage가 이미 설치되어 있습니다."
fi

# QEMU 설치 확인
echo ""
echo "[5/7] QEMU 설치 확인 중..."
if ! command -v qemu-system-x86_64 &> /dev/null; then
    echo "QEMU가 설치되어 있지 않습니다."
    echo "QEMU를 설치하려면 다음 명령을 실행하세요:"
    echo "  Ubuntu/Debian: sudo apt-get install qemu-system-x86 qemu-utils ovmf"
    echo "  Fedora: sudo dnf install qemu-system-x86 qemu-utils edk2-ovmf"
else
    echo "QEMU가 설치되어 있습니다: $(which qemu-system-x86_64)"
fi

# NASM 설치 확인
echo ""
echo "[6/7] NASM 설치 확인 중..."
if ! command -v nasm &> /dev/null; then
    echo "NASM이 설치되어 있지 않습니다 (선택적)."
    echo "NASM을 설치하려면: sudo apt-get install nasm (Ubuntu/Debian)"
else
    echo "NASM이 설치되어 있습니다: $(which nasm)"
fi

# GDB 설치 확인
echo ""
echo "[7/7] GDB 설치 확인 중..."
if ! command -v gdb &> /dev/null; then
    echo "GDB가 설치되어 있지 않습니다 (선택적, 디버깅용)."
    echo "GDB를 설치하려면: sudo apt-get install gdb (Ubuntu/Debian)"
else
    echo "GDB가 설치되어 있습니다: $(which gdb)"
fi

# 최종 검증
echo ""
echo "=== 설정 완료 ==="
echo ""
echo "설치된 도구 확인:"
rustup show
echo ""
echo "설치된 타겟 확인:"
rustup target list --installed
echo ""
echo "다음 단계:"
echo "  1. 'cargo build' 를 실행하여 프로젝트가 빌드되는지 확인하세요."
echo "  2. './scripts/run.sh' 를 실행하여 QEMU에서 커널을 테스트하세요."

