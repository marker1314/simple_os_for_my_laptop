#!/bin/bash
# Simple OS 전력 테스트 스크립트
# 10분간 idle 상태에서 전력 측정을 수행합니다.

set -e

echo "=== Simple OS 전력 테스트 (10분 idle) ==="
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
POWER_CSV="power_idle_$(date +%Y%m%d_%H%M%S).csv"
LOG_FILE="power_test_$(date +%Y%m%d_%H%M%S).log"

echo ""
echo "[2/3] QEMU에서 커널 실행 중..."
echo "전력 통계 CSV: $POWER_CSV"
echo "전체 로그: $LOG_FILE"
echo ""
echo "테스트: 10분간 idle 상태에서 전력 측정"
echo "종료하려면 Ctrl+C를 누르세요."
echo ""

BOOTIMAGE_PATH="target/x86_64-unknown-none/debug/bootimage-simple_os.bin"

if [ ! -f "$BOOTIMAGE_PATH" ]; then
    echo "오류: 부팅 이미지를 찾을 수 없습니다: $BOOTIMAGE_PATH"
    exit 1
fi

# QEMU 실행 (시리얼 출력을 파일로 저장)
qemu-system-x86_64 \
    -drive format=raw,file="$BOOTIMAGE_PATH" \
    -serial stdio 2>&1 | tee "$LOG_FILE" &

QEMU_PID=$!

# 10분 대기 (600초)
echo "[3/3] 10분간 전력 측정 중..."
sleep 600

# QEMU 종료
echo ""
echo "테스트 완료. QEMU 종료 중..."
kill $QEMU_PID 2>/dev/null || true

# CSV 추출 (로그에서 power 관련 라인만 추출)
echo "CSV 파일 생성 중..."
grep -E "timestamp|pkg_w|core_cstate|wakeups" "$LOG_FILE" > "$POWER_CSV" || echo "timestamp,pkg_w,core_cstate_residency,wakeups_per_s" > "$POWER_CSV"

echo ""
echo "=== 테스트 완료 ==="
echo "전력 통계 CSV: $POWER_CSV"
echo "전체 로그: $LOG_FILE"

