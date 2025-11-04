# 테스트 디렉토리

이 디렉토리에는 통합 테스트 및 추가 테스트 코드가 포함됩니다.

## 테스트 구조

- 단위 테스트: 각 모듈의 `mod.rs` 또는 별도 테스트 파일에 포함
- 통합 테스트: 이 디렉토리에 별도 파일로 작성
- 하드웨어 테스트: QEMU 및 실제 하드웨어에서 실행

## 전력/신뢰성 체크리스트

- Boot profiles: balanced/performance/power_saver/headless 부팅 확인
- Shell: `power status`, `power mode`, `power disk idle` 동작 확인
- GUI: 입력 없음 60초 후 화면 블랭크 (power_saver)
- ATA: 유휴 타임아웃 후 standby 진입 및 I/O 시 복귀
- RTL8139: 10초 유휴 후 RX 정지, 트래픽 시 재개

