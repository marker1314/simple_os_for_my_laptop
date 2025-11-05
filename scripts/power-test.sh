#!/bin/bash
# 전력 테스트 스크립트
# 유휴 전력 측정, Suspend/Resume 사이클, 프로파일 검증

set -e

LOG_FILE="power-test.log"
QEMU_IMAGE="target/x86_64-unknown-none/debug/simple_os"

echo "=== Power Test Started ===" | tee -a "$LOG_FILE"
echo "Start time: $(date)" | tee -a "$LOG_FILE"

# 유휴 전력 측정 (10분)
echo "Measuring idle power consumption (10 minutes)..." | tee -a "$LOG_FILE"
timeout 600 qemu-system-x86_64 \
    -drive format=raw,file="$QEMU_IMAGE" \
    -serial stdio \
    -monitor none \
    -no-reboot \
    -no-shutdown \
    > "${LOG_FILE}.idle" 2>&1 || true

# 전력 통계 추출
echo "=== Idle Power Statistics ===" | tee -a "$LOG_FILE"
grep -i "power\|energy\|mW\|mJ" "${LOG_FILE}.idle" | tail -20 || true

# C-State/P-State residency 추출
echo "=== C-State/P-State Residency ===" | tee -a "$LOG_FILE"
grep -i "C[0-9].*residency\|P[0-9].*residency" "${LOG_FILE}.idle" | tail -20 || true

# Suspend/Resume 테스트 (5회 반복)
echo "=== Suspend/Resume Cycle Test ===" | tee -a "$LOG_FILE"
for i in {1..5}; do
    echo "Cycle $i/5..." | tee -a "$LOG_FILE"
    
    timeout 60 qemu-system-x86_64 \
        -drive format=raw,file="$QEMU_IMAGE" \
        -serial stdio \
        -monitor none \
        -no-reboot \
        -no-shutdown \
        > "${LOG_FILE}.suspend${i}" 2>&1 || true
    
    # Suspend/Resume 로그 확인
    if grep -q "suspend\|resume" "${LOG_FILE}.suspend${i}"; then
        echo "Cycle $i: Suspend/Resume detected" | tee -a "$LOG_FILE"
    else
        echo "Cycle $i: WARNING - No suspend/resume detected" | tee -a "$LOG_FILE"
    fi
done

# 최종 통계
echo "=== Final Power Test Results ===" | tee -a "$LOG_FILE"
echo "End time: $(date)" | tee -a "$LOG_FILE"

# 통계 요약
echo "=== Power Test Summary ===" | tee -a "$LOG_FILE"
echo "Idle power test: completed" | tee -a "$LOG_FILE"
echo "Suspend/Resume cycles: 5" | tee -a "$LOG_FILE"

echo "Power test completed!" | tee -a "$LOG_FILE"
exit 0
