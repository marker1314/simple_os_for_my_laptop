#!/bin/bash
# Simple OS 베이스라인 측정 스크립트
# 부팅 타임라인, 전력 통계, 크래시 덤프를 수집합니다.

set -e

echo "=== Simple OS 베이스라인 측정 ==="
echo ""

# 현재 디렉토리가 프로젝트 루트인지 확인
if [ ! -f "Cargo.toml" ]; then
    echo "오류: Cargo.toml을 찾을 수 없습니다. 프로젝트 루트에서 실행하세요."
    exit 1
fi

# 출력 디렉토리
OUTPUT_DIR="baseline_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUTPUT_DIR"

echo "출력 디렉토리: $OUTPUT_DIR"
echo ""

# bootimage로 부팅 이미지 생성
echo "[1/4] 커널 빌드 중..."
cargo bootimage 2>&1 | tee "$OUTPUT_DIR/build.log"
if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo "빌드 실패!"
    exit 1
fi
echo "빌드 완료!"
echo ""

# 출력 파일
BOOT_TIMELINE_CSV="$OUTPUT_DIR/boot_timeline.csv"
POWER_CSV="$OUTPUT_DIR/power_idle.csv"
LOG_FILE="$OUTPUT_DIR/baseline.log"
CRASH_DUMP="$OUTPUT_DIR/crash_dump.txt"

echo "[2/4] QEMU에서 커널 실행 중..."
echo "로그 파일: $LOG_FILE"
echo "부팅 타임라인: $BOOT_TIMELINE_CSV"
echo "전력 통계: $POWER_CSV"
echo ""
echo "10분간 측정합니다. Ctrl+C로 중단할 수 있습니다."
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
echo "[3/4] 10분간 측정 중..."
sleep 600

# QEMU 종료
echo ""
echo "[4/4] 측정 완료. QEMU 종료 중..."
kill $QEMU_PID 2>/dev/null || true
wait $QEMU_PID 2>/dev/null || true

# CSV 추출
echo ""
echo "CSV 파일 추출 중..."

# 부팅 타임라인 추출
if grep -q "Boot Timeline" "$LOG_FILE" || grep -q "boot_timeline" "$LOG_FILE"; then
    # boot_timeline.csv 형식 찾기
    grep -A 1000 "stage,timestamp_ms,relative_ms" "$LOG_FILE" | head -50 > "$BOOT_TIMELINE_CSV" 2>/dev/null || \
    echo "stage,timestamp_ms,relative_ms" > "$BOOT_TIMELINE_CSV"
else
    echo "stage,timestamp_ms,relative_ms" > "$BOOT_TIMELINE_CSV"
    echo "Warning: 부팅 타임라인 데이터를 찾을 수 없습니다."
fi

# 전력 통계 추출
if grep -q "timestamp,pkg_w" "$LOG_FILE"; then
    grep -A 1000 "timestamp,pkg_w" "$LOG_FILE" | head -100 > "$POWER_CSV" 2>/dev/null || \
    echo "timestamp,pkg_w,core_cstate_residency,wakeups_per_s" > "$POWER_CSV"
else
    echo "timestamp,pkg_w,core_cstate_residency,wakeups_per_s" > "$POWER_CSV"
    echo "Warning: 전력 통계 데이터를 찾을 수 없습니다."
fi

# 크래시 덤프 추출
if grep -q "Crash Dump" "$LOG_FILE"; then
    grep -A 50 "Crash Dump" "$LOG_FILE" > "$CRASH_DUMP" 2>/dev/null || true
fi

# 요약 리포트 생성
SUMMARY="$OUTPUT_DIR/summary.txt"
{
    echo "=== Simple OS 베이스라인 측정 요약 ==="
    echo "측정 시간: $(date)"
    echo ""
    
    # 부팅 타임라인 요약
    if [ -f "$BOOT_TIMELINE_CSV" ] && [ $(wc -l < "$BOOT_TIMELINE_CSV") -gt 1 ]; then
        echo "--- 부팅 타임라인 ---"
        tail -1 "$BOOT_TIMELINE_CSV" | awk -F',' '{print "총 부팅 시간: " $3 "ms"}'
        echo ""
    fi
    
    # 전력 통계 요약
    if [ -f "$POWER_CSV" ] && [ $(wc -l < "$POWER_CSV") -gt 1 ]; then
        echo "--- 전력 통계 ---"
        tail -1 "$POWER_CSV" | awk -F',' '{print "평균 전력: " $2 "mW"}'
        tail -1 "$POWER_CSV" | awk -F',' '{print "C-State Residency: " $3 "%"}'
        tail -1 "$POWER_CSV" | awk -F',' '{print "Wakeup Rate: " $4 "/s"}'
        echo ""
    fi
    
    # 크래시 정보
    if [ -f "$CRASH_DUMP" ] && [ -s "$CRASH_DUMP" ]; then
        echo "--- 크래시 정보 ---"
        echo "크래시 발생: 예"
        echo "자세한 내용은 $CRASH_DUMP 참조"
        echo ""
    else
        echo "--- 크래시 정보 ---"
        echo "크래시 발생: 아니오"
        echo ""
    fi
    
    echo "=== 측정 완료 ==="
    echo "결과 파일: $OUTPUT_DIR/"
} > "$SUMMARY"

cat "$SUMMARY"

echo ""
echo "=== 측정 완료 ==="
echo "결과 파일: $OUTPUT_DIR/"
echo "  - 부팅 타임라인: $BOOT_TIMELINE_CSV"
echo "  - 전력 통계: $POWER_CSV"
echo "  - 전체 로그: $LOG_FILE"
echo "  - 크래시 덤프: $CRASH_DUMP"
echo "  - 요약: $SUMMARY"



