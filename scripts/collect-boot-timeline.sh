#!/bin/bash
# 부팅 타임라인 수집 스크립트
# QEMU 실행 중 부팅 타임라인 CSV를 추출합니다.

set -e

if [ $# -lt 1 ]; then
    echo "사용법: $0 <log_file> [output_file]"
    echo "  log_file: QEMU 시리얼 출력 로그 파일"
    echo "  output_file: 출력 CSV 파일 (기본값: boot_timeline.csv)"
    exit 1
fi

LOG_FILE="$1"
OUTPUT_FILE="${2:-boot_timeline.csv}"

if [ ! -f "$LOG_FILE" ]; then
    echo "오류: 로그 파일을 찾을 수 없습니다: $LOG_FILE"
    exit 1
fi

echo "부팅 타임라인 추출 중..."
echo "입력: $LOG_FILE"
echo "출력: $OUTPUT_FILE"

# 부팅 타임라인 CSV 추출
if grep -q "stage,timestamp_ms,relative_ms" "$LOG_FILE"; then
    # CSV 헤더와 데이터 추출
    grep -A 1000 "stage,timestamp_ms,relative_ms" "$LOG_FILE" | \
        grep -E "^[A-Za-z]+,|^[A-Za-z]+[A-Za-z0-9]+,|^[0-9]+," | \
        head -50 > "$OUTPUT_FILE"
    
    LINE_COUNT=$(wc -l < "$OUTPUT_FILE")
    if [ "$LINE_COUNT" -gt 1 ]; then
        echo "성공: $LINE_COUNT 라인 추출됨"
        echo ""
        echo "첫 5줄:"
        head -5 "$OUTPUT_FILE"
    else
        echo "경고: 데이터가 없습니다."
        echo "stage,timestamp_ms,relative_ms" > "$OUTPUT_FILE"
    fi
else
    echo "경고: 부팅 타임라인 CSV를 찾을 수 없습니다."
    echo "stage,timestamp_ms,relative_ms" > "$OUTPUT_FILE"
fi

