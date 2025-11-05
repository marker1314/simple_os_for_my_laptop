# ELAN I2C-HID 트랙패드 지원 가이드

## 개요

이 OS는 HP 14s-dk0112AU 및 유사한 노트북의 ELAN I2C-HID 트랙패드를 지원합니다.

### 지원되는 장치

- **ELAN708:00 04F3:30A0** (HP 14s-dk0112AU)
- 기타 ELAN I2C-HID 호환 트랙패드

## 아키텍처

```
애플리케이션 (GUI)
    ↓
MouseEvent 통합 레이어
    ↓
┌──────────────┬──────────────────┐
│  PS/2 마우스  │  I2C-HID 트랙패드  │
└──────────────┴──────────────────┘
       ↓                ↓
    mouse.rs        touchpad.rs
                       ↓
                   i2c_hid.rs
                       ↓
                    i2c.rs
                       ↓
                AMD FCH I2C 컨트롤러
```

## 구현된 모듈

### 1. I2C 버스 컨트롤러 (`src/drivers/i2c.rs`)
- AMD FCH I2C 컨트롤러 지원
- MMIO 기반 레지스터 접근
- START/STOP 조건 생성
- 바이트 단위 읽기/쓰기
- ACK/NACK 처리

### 2. I2C-HID 프로토콜 (`src/drivers/i2c_hid.rs`)
- I2C-HID 사양 1.00 구현
- HID Descriptor 파싱
- Input/Output Report 처리
- Power 관리 (On/Sleep)
- 장치 리셋

### 3. ELAN 트랙패드 드라이버 (`src/drivers/touchpad.rs`)
- ELAN 특화 Input Report 파싱
- 절대 좌표 → 상대 좌표 변환
- 터치 이벤트 → MouseEvent 변환
- 멀티터치 기본 지원 구조
- 화면 경계 처리

### 4. ACPI I2C 장치 검색 (`src/power/acpi.rs`)
- I2C 컨트롤러 베이스 주소 제공
- I2C 슬레이브 주소 제공
- HID (Hardware ID) 정보

## 하드웨어 설정

### I2C 컨트롤러 주소
- **베이스 주소**: `0xFEDC3000` (AMD FCH I2C Controller 0)
- **슬레이브 주소**: `0x15` (ELAN 트랙패드)

### 레지스터 맵 (AMD FCH I2C)
```
Offset  | Name              | Description
--------|-------------------|---------------------------
0x00    | Control           | 제어 레지스터
0x04    | Status            | 상태 레지스터
0x08    | SlaveAddress      | 슬레이브 주소
0x0C    | Data              | 데이터 레지스터
0x10    | Control2          | 확장 제어
0x14    | InterruptControl  | 인터럽트 제어
```

### I2C-HID Descriptor 구조
```rust
struct I2cHidDescriptor {
    length: u16,                    // 30 bytes
    bcd_version: u16,               // 0x0100 (1.00)
    report_desc_length: u16,        // Report Descriptor 길이
    report_desc_register: u16,      // Report Descriptor 레지스터
    input_register: u16,            // Input Report 레지스터
    max_input_length: u16,          // 최대 입력 길이
    output_register: u16,           // Output Report 레지스터
    max_output_length: u16,         // 최대 출력 길이
    command_register: u16,          // 명령 레지스터
    data_register: u16,             // 데이터 레지스터
    vendor_id: u16,                 // 0x04F3 (ELAN)
    product_id: u16,                // 0x30A0 (ELAN708)
    version_id: u16,                // 버전
    reserved: u32,                  // 예약됨
}
```

## 초기화 순서

1. **메모리 관리 초기화** (필수)
2. **ACPI에서 I2C 정보 읽기**
   ```rust
   let i2c_info = acpi::find_i2c_touchpad();
   ```
3. **I2C 컨트롤러 초기화**
   ```rust
   i2c::init_controller(0, i2c_info.base_address);
   ```
4. **트랙패드 초기화**
   ```rust
   touchpad::init(i2c_info.slave_address);
   ```
5. **GUI 루프에서 이벤트 폴링**
   ```rust
   if let Some(event) = touchpad::poll_event() {
       desktop_manager::handle_mouse_event(event);
   }
   ```

## 사용 방법

### GUI 모드에서
트랙패드는 자동으로 감지되고 초기화됩니다. GUI 데스크톱 환경에서:
- **터치**: 마우스 이동 (Move 이벤트)
- **탭**: 왼쪽 클릭 (LeftButtonDown/Up 이벤트)
- PS/2 마우스와 트랙패드 동시 사용 가능

### 이벤트 흐름
```
터치패드 하드웨어
    ↓ (I2C 인터럽트 또는 폴링)
I2C-HID Input Report
    ↓ (파싱)
TouchPoint { x, y, pressure }
    ↓ (변환)
MouseEvent::Move(cursor_x, cursor_y)
    ↓
GUI Desktop Manager
    ↓
애플리케이션 (Calculator, File Manager 등)
```

## 트러블슈팅

### 트랙패드가 작동하지 않는 경우

1. **I2C 컨트롤러 베이스 주소 확인**
   ```bash
   # Linux에서 확인
   sudo cat /sys/firmware/acpi/tables/DSDT > dsdt.dat
   iasl -d dsdt.dat
   grep -A 20 "I2C" dsdt.dsl
   ```

2. **I2C 슬레이브 주소 확인**
   ```bash
   sudo i2cdetect -l
   sudo i2cdetect -y 0
   ```

3. **로그 확인**
   - "I2C controller initialized" 메시지 확인
   - "ELAN touchpad initialized" 메시지 확인
   - 에러 메시지 확인

### 일반적인 문제

#### 문제: NACK 에러
**원인**: I2C 슬레이브 주소가 잘못됨  
**해결**: `acpi.rs`의 `slave_address` 값 수정 (0x15, 0x2C 등 시도)

#### 문제: 타임아웃 에러
**원인**: I2C 컨트롤러 베이스 주소가 잘못됨  
**해결**: ACPI DSDT에서 실제 주소 확인 후 수정

#### 문제: 트랙패드가 감지되지만 응답 없음
**원인**: HID Descriptor 읽기 실패  
**해결**: I2C 클럭 속도 조정 또는 대기 시간 증가

## 확장 가능성

### 멀티터치 지원
현재 구조는 멀티터치를 지원할 수 있도록 설계되었습니다:
```rust
struct TouchPoint {
    x: u16,
    y: u16,
    touching: bool,
    pressure: u8,
    // 추가 가능:
    // finger_id: u8,
    // width: u16,
    // height: u16,
}
```

### 제스처 인식
터치 이벤트 시퀀스를 분석하여 제스처 감지 가능:
- 두 손가락 스크롤
- 핀치 줌
- 스와이프

### 다른 I2C-HID 장치
같은 드라이버 스택으로 다른 I2C-HID 장치 지원 가능:
- Synaptics 트랙패드
- Goodix 터치스크린
- Wacom 디지타이저

## 성능 최적화

### 폴링 vs 인터럽트
현재는 **폴링 방식**을 사용:
- 60 FPS 렌더링 루프에서 폴링
- CPU 사용률: 낮음 (HLT 명령 사용)

인터럽트 방식으로 전환 가능:
1. I2C 인터럽트 활성화
2. IDT에 핸들러 등록
3. 이벤트 큐 구현

### 응답성
- **폴링 간격**: 16ms (60 FPS)
- **입력 지연**: < 20ms
- **터치 감도**: 압력 임계값 조정 가능

## 참고 자료

### 사양 문서
- [I2C-HID Specification v1.00](https://docs.microsoft.com/en-us/windows-hardware/design/component-guidelines/hid-over-i2c-protocol-spec)
- [USB HID Specification](https://www.usb.org/hid)
- [AMD FCH Technical Reference Manual](https://www.amd.com/en/support/tech-docs)

### 유사 프로젝트
- Linux Kernel `drivers/hid/i2c-hid/`
- FreeBSD I2C-HID 드라이버
- Haiku OS I2C 드라이버

## 라이선스 및 기여

이 트랙패드 드라이버는 프로젝트의 나머지 부분과 동일한 라이선스를 따릅니다.

기여 환영:
- 다른 ELAN 모델 테스트
- Synaptics/Goodix 지원 추가
- 멀티터치 구현
- 제스처 인식

---

**문의사항이나 버그 리포트는 GitHub Issues에 등록해주세요.**


