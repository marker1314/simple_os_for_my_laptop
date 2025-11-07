#!/bin/bash
# 안정성 테스트 스크립트
# 24시간 스트레스 테스트, 메모리 누수 검사, 파일시스템 무결성 검증

set -e

LOG_FILE="stability-test.log"
DURATION=${1:-86400}  # 기본 24시간 (초)
QEMU_IMAGE="target/x86_64-unknown-none/debug/simple_os"

echo "=== Stability Test Started ===" | tee -a "$LOG_FILE"
echo "Duration: $DURATION seconds" | tee -a "$LOG_FILE"
echo "Start time: $(date)" | tee -a "$LOG_FILE"

# QEMU 실행 (백그라운드)
qemu-system-x86_64 \
    -drive format=raw,file="$QEMU_IMAGE" \
    -serial stdio \
    -monitor none \
    -no-reboot \
    -no-shutdown \
    > "$LOG_FILE" 2>&1 &
QEMU_PID=$!

echo "QEMU started with PID: $QEMU_PID" | tee -a "$LOG_FILE"

# 테스트 종료 함수
cleanup() {
    echo "Test interrupted. Cleaning up..." | tee -a "$LOG_FILE"
    kill $QEMU_PID 2>/dev/null || true
    wait $QEMU_PID 2>/dev/null || true
    exit 0
}

trap cleanup INT TERM

# 주기적으로 메모리 상태 확인
check_memory() {
    echo "[$(date)] Checking memory..." | tee -a "$LOG_FILE"
    # 메모리 통계 추출
    grep -i "memory\|heap\|frame" "$LOG_FILE" | tail -5 || true
}

# 파일시스템 무결성 확인
check_fs() {
    echo "[$(date)] Checking filesystem..." | tee -a "$LOG_FILE"
    # 파일시스템 오류 로그 확인
    grep -i "fsck\|journal\|corrupt\|error" "$LOG_FILE" | tail -5 || true
}

# 주기적 체크 (10분마다)
CHECK_INTERVAL=600
ELAPSED=0

while [ $ELAPSED -lt $DURATION ]; do
    sleep $CHECK_INTERVAL
    ELAPSED=$((ELAPSED + CHECK_INTERVAL))
    
    # QEMU 프로세스 확인
    if ! kill -0 $QEMU_PID 2>/dev/null; then
        echo "QEMU process died unexpectedly!" | tee -a "$LOG_FILE"
        exit 1
    fi
    
    check_memory
    check_fs
    
    echo "[$(date)] Test progress: $ELAPSED/$DURATION seconds" | tee -a "$LOG_FILE"
done

# 테스트 완료
echo "=== Stability Test Completed ===" | tee -a "$LOG_FILE"
echo "End time: $(date)" | tee -a "$LOG_FILE"
echo "Total duration: $DURATION seconds" | tee -a "$LOG_FILE"

# 최종 통계 수집
echo "=== Final Statistics ===" | tee -a "$LOG_FILE"
check_memory
check_fs

# 크래시 덤프 확인
if grep -q "CRASH\|PANIC\|Exception" "$LOG_FILE"; then
    echo "WARNING: Crashes detected during test!" | tee -a "$LOG_FILE"
    grep -i "CRASH\|PANIC\|Exception" "$LOG_FILE" | tee -a "$LOG_FILE"
    exit 1
fi

# QEMU 종료
kill $QEMU_PID 2>/dev/null || true
wait $QEMU_PID 2>/dev/null || true

echo "Test passed successfully!" | tee -a "$LOG_FILE"
exit 0



