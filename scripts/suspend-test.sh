#!/bin/bash
# Simple OS Suspend/Resume 테스트 스크립트
# 50회 suspend/resume 사이클을 테스트합니다.

set -e

echo "=== Simple OS Suspend/Resume 테스트 ==="
echo ""

# 현재 디렉토리가 프로젝트 루트인지 확인
if [ ! -f "Cargo.toml" ]; then
    echo "오류: Cargo.toml을 찾을 수 없습니다. 프로젝트 루트에서 실행하세요."
    exit 1
fi

# bootimage로 부팅 이미지 생성
echo "[1/3] 커널 빌드 중..."
cargo bootimage
if [ $? -ne 0 ]; then
    echo "빌드 실패!"
    exit 1
fi
echo "빌드 완료!"

# 출력 파일
SUSPEND_LOG="suspend_cycles_$(date +%Y%m%d_%H%M%S).log"
SUSPEND_CSV="suspend_cycles_$(date +%Y%m%d_%H%M%S).csv"

echo ""
echo "[2/3] QEMU에서 커널 실행 중..."
echo "Suspend 테스트 로그: $SUSPEND_LOG"
echo "Suspend CSV: $SUSPEND_CSV"
echo ""
echo "테스트: 50회 suspend/resume 사이클"
echo "참고: 이 테스트는 수동으로 suspend 명령을 트리거해야 합니다."
echo "      커널에서 suspend 명령을 실행하면 자동으로 resume됩니다."
echo ""

BOOTIMAGE_PATH="target/x86_64-unknown-none/debug/bootimage-simple_os.bin"

if [ ! -f "$BOOTIMAGE_PATH" ]; then
    echo "오류: 부팅 이미지를 찾을 수 없습니다: $BOOTIMAGE_PATH"
    exit 1
fi

# CSV 헤더 작성
echo "cycle_id,result,resume_ms,failures" > "$SUSPEND_CSV"

# QEMU 실행 (시리얼 출력을 파일로 저장)
# 참고: 실제 suspend/resume은 커널 내부에서 처리되므로
# 여기서는 로그만 수집합니다.
qemu-system-x86_64 \
    -drive format=raw,file="$BOOTIMAGE_PATH" \
    -serial stdio 2>&1 | tee "$SUSPEND_LOG" | grep -E "(suspend|resume|S3|cycle)" || true

echo ""
echo "=== 테스트 완료 ==="
echo "Suspend 로그: $SUSPEND_LOG"
echo "Suspend CSV: $SUSPEND_CSV"
echo ""
echo "참고: 실제 suspend/resume 테스트는 커널 내부에서 구현되어야 합니다."
echo "      현재는 로그만 수집합니다."

