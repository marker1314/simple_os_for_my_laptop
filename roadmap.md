# Rust 기반 초저전력 노트북 OS 개발 로드맵

## 목차
0. [프로젝트 시작 전 결정 사항](#0-프로젝트-시작-전-결정-사항)
1. [전체 방향 정의 및 아키텍처 설계](#1-전체-방향-정의-및-아키텍처-설계)
2. [Rust OS 개발 환경 구축](#2-rust-os-개발-환경-구축)
3. [부트로더 및 커널 초기화](#3-부트로더-및-커널-초기화)
4. [커널 코어 구성](#4-커널-코어-구성)
5. [전력 관리 시스템](#5-전력-관리-시스템)
6. [하드웨어 드라이버 구현](#6-하드웨어-드라이버-구현)
7. [빌드 시스템 및 부팅 테스트](#7-빌드-시스템-및-부팅-테스트)
8. [Shell 및 GUI 시스템](#8-shell-및-gui-시스템)
9. [고급 기능 및 최적화](#9-고급-기능-및-최적화)
10. [참고 OS 프로젝트 및 학습 자료](#10-참고-os-프로젝트-및-학습-자료)

---

## 0. 프로젝트 시작 전 결정 사항

이 섹션은 프로젝트를 시작하기 전에 반드시 결정해야 할 사항들을 정리한 것입니다. 이러한 결정들은 이후 모든 개발 과정에 영향을 미치므로 신중하게 결정해야 합니다.

### 0.1 프로젝트 정책 및 라이선스

**라이선스 선택:**
- [ ] **MIT License** - 가장 관대한 라이선스, 상업적 사용 가능
- [ ] **Apache 2.0** - 특허 보호 포함, Apache 프로젝트 표준
- [ ] **GPL v2/v3** - Copyleft, 파생작품도 GPL 적용
- [ ] **BSD 2-Clause/3-Clause** - MIT와 유사하지만 조항 차이
- [ ] **이중 라이선스** - 예: Apache 2.0 + MIT
- [ ] **커스텀 라이선스** - 특수 요구사항이 있는 경우

**오픈소스 전략:**
- [ ] 완전 오픈소스 (공개 레포지토리)
- [ ] 부분 오픈소스 (코어만 공개)
- [ ] 프라이빗 개발 후 후기 공개
- [ ] 상업적 라이선스 옵션 제공

**기여 정책:**
- [ ] CONTRIBUTING.md 작성 여부
- [ ] 코드 리뷰 프로세스 정의
- [ ] 기여자 라이선스 협약 (CLA) 필요 여부

### 0.2 프로젝트 관리 및 워크플로우

**버전 관리:**
- [ ] Git 저장소 호스팅 (GitHub, GitLab, Gitea 등)
- [ ] 브랜치 전략 (Git Flow, GitHub Flow, Trunk-based 등)
- [ ] 커밋 메시지 컨벤션 (Conventional Commits 등)
- [ ] 태그/릴리즈 관리 전략

**이슈 관리:**
- [ ] 이슈 템플릿 정의
- [ ] 라벨 시스템 설계 (bug, feature, enhancement 등)
- [ ] 프로젝트 보드 사용 여부

**문서화 언어:**
- [ ] 한국어 (현재 로드맵 기준)
- [ ] 영어 (국제적 접근성)
- [ ] 이중 언어 (한국어 + 영어)

**코드 스타일:**
- [ ] `rustfmt.toml` 설정
- [ ] `clippy.toml` 설정 (린터 규칙)
- [ ] 코드 리뷰 체크리스트

### 0.3 타겟 하드웨어 명시

**특정 노트북 모델 결정:**
- [ ] 테스트 대상 노트북 모델 선택
  - 예: Dell XPS 13, Lenovo ThinkPad X1, MacBook (Boot Camp) 등
- [ ] CPU 모델: Intel (몇 세대?), AMD Ryzen (몇 세대?)
- [ ] 칩셋: Intel, AMD, 기타
- [ ] UEFI 펌웨어 버전 확인 필요 여부

**하드웨어 우선순위:**
- [ ] 필수 지원 하드웨어 목록
- [ ] 선택적 지원 하드웨어 목록
- [ ] 호환성 목표 (예: Intel 8세대 이상, AMD Ryzen 3000 이상)

**하드웨어 접근:**
- [ ] 테스트용 전용 노트북 확보 여부
- [ ] 가상화 우선 개발 후 실제 하드웨어 테스트

### 0.4 개발 범위 및 우선순위

**MVP (Minimum Viable Product) 정의:**
- [ ] 첫 번째 목표: 단순 부팅 + 텍스트 출력
- [ ] 두 번째 목표: 기본 Shell 동작
- [ ] 세 번째 목표: 전력 관리 기본 기능
- [ ] 기능별 우선순위 매트릭스 작성

**범위 제한:**
- [ ] 지원하지 않을 기능 명시
  - 예: 그래픽 가속, 멀티미디어 코덱, 특정 파일시스템 등
- [ ] 단계별 범위 조정 계획

**성능 목표 구체화:**
- [ ] 부팅 시간 목표: ___ 초
- [ ] 유휴 전력 소비 목표: ___ W
- [ ] 메모리 사용량 목표: ___ MB (최소), ___ MB (권장)
- [ ] CPU 사용률 목표 (유휴 시): ___ %
- [ ] 측정 방법 및 벤치마크 도구 선택

### 0.5 호환성 및 표준

**POSIX 호환성:**
- [ ] 부분 POSIX 호환 목표
- [ ] 완전 비호환 (독자적 API)
- [ ] 호환성 레이어 후기 추가 가능성

**실행 파일 포맷:**
- [ ] ELF 바이너리 지원
- [ ] 커스텀 바이너리 포맷
- [ ] 바이너리 호환성 목표 (예: Linux ELF 호환)

**파일시스템:**
- [ ] 지원할 파일시스템 우선순위
  - FAT32 (필수/선택)
  - Ext2/3/4 (필수/선택)
  - 커스텀 파일시스템 (필수/선택)

**네트워크 프로토콜:**
- [ ] IPv4/IPv6 지원 여부
- [ ] TCP/UDP 우선순위
- [ ] 네트워크 표준 호환 목표

### 0.6 빌드 및 배포 전략

**빌드 타겟:**
- [ ] 디버그 빌드 최적화 레벨
- [ ] 릴리즈 빌드 최적화 레벨
- [ ] 다양한 하드웨어용 빌드 매트릭스

**배포 방법:**
- [ ] ISO 이미지 생성
- [ ] USB 부팅 이미지
- [ ] 네트워크 부팅 (PXE) 지원 여부
- [ ] 설치 프로그램 제공 여부

**버전 관리:**
- [ ] 시맨틱 버저닝 (Semantic Versioning) 사용 여부
- [ ] 버전 번호 체계 (예: 0.1.0, 1.0.0 등)
- [ ] 안정성 마일스톤 정의

**CI/CD 파이프라인:**
- [ ] 자동 빌드 (GitHub Actions, GitLab CI 등)
- [ ] 자동 테스트 실행
- [ ] 자동 배포 프로세스

### 0.7 에러 처리 및 로깅 전략

**에러 처리 방식:**
- [ ] `Result<T, E>` 기반 에러 처리
- [ ] 커스텀 에러 타입 정의 범위
- [ ] 패닉 정책 (언제 패닉을 허용할지)
- [ ] 복구 가능/불가능 에러 구분

**로깅 시스템:**
- [ ] 로그 레벨 정의 (ERROR, WARN, INFO, DEBUG, TRACE)
- [ ] 로그 출력 위치 (시리얼, 파일, 네트워크 등)
- [ ] 로그 포맷 (구조화된 로그, 텍스트 로그)
- [ ] 성능 영향 최소화 전략

**디버깅 지원:**
- [ ] 커널 디버거 통합 (KGDB 등)
- [ ] 스택 트레이스 출력 형식
- [ ] 메모리 덤프 기능
- [ ] 커널 프로파일링 도구

### 0.8 보안 정책

**보안 목표:**
- [ ] 커널 레벨 보안 요구사항
- [ ] 사용자 공간 격리 목표
- [ ] 권한 관리 (root/non-root)
- [ ] ASLR, 스택 카나리 등 보안 기능 우선순위

**취약점 관리:**
- [ ] 보안 이슈 보고 프로세스
- [ ] CVE 추적 및 패치 전략
- [ ] 보안 감사 주기

### 0.9 테스트 전략

**테스트 범위:**
- [ ] 단위 테스트 커버리지 목표 (예: 70% 이상)
- [ ] 통합 테스트 범위
- [ ] 하드웨어 호환성 테스트 계획
- [ ] 성능 테스트 벤치마크

**테스트 도구:**
- [ ] 커널 단위 테스트 프레임워크
- [ ] 통합 테스트 환경 (QEMU 스크립트 등)
- [ ] 자동화된 테스트 실행

**테스트 데이터:**
- [ ] 테스트용 샘플 파일/데이터
- [ ] 테스트 케이스 문서화

### 0.10 의존성 관리

**크레이트 선택 원칙:**
- [ ] 최소 의존성 원칙
- [ ] 크레이트 검토 기준 (메인테넌스 상태, 라이선스 등)
- [ ] 커스텀 포크 허용 여부
- [ ] 의존성 업데이트 정책

**보안 취약점 관리:**
- [ ] `cargo audit` 사용 여부
- [ ] 의존성 취약점 모니터링
- [ ] 업데이트 주기

### 0.11 커뮤니티 및 문서화

**커뮤니티 관리:**
- [ ] 커뮤니티 채널 (Discord, Matrix, 포럼 등)
- [ ] 행동 강령 (Code of Conduct) 작성 여부
- [ ] 커뮤니티 가이드라인

**문서화 범위:**
- [ ] API 문서 자동 생성 (`cargo doc`)
- [ ] 사용자 매뉴얼 작성
- [ ] 개발자 가이드 작성
- [ ] 아키텍처 문서 유지
- [ ] 튜토리얼/예제 코드

### 0.12 개발 리소스 및 일정

**개발 환경:**
- [ ] 주 개발 OS (Windows/Linux/macOS)
- [ ] 개발 머신 사양 요구사항
- [ ] 필수 도구 설치 완료 여부

**개발 일정:**
- [ ] 마일스톤 및 데드라인 설정
- [ ] 주당 개발 시간 예상
- [ ] 단계별 목표 기간 설정

**리소스 관리:**
- [ ] 프로젝트 예산 (하드웨어 구매 등)
- [ ] 클라우드 리소스 필요 여부 (CI/CD 등)

### 0.13 결정 체크리스트

프로젝트를 시작하기 전에 다음 항목들을 확인하세요:

**필수 결정 사항:**
- [ ] 라이선스 선택 완료
- [ ] 타겟 하드웨어 모델 결정
- [ ] 개발 환경 구축 완료
- [ ] 버전 관리 저장소 설정
- [ ] MVP 범위 정의
- [ ] 성능 목표 수치 설정

**권장 결정 사항:**
- [ ] 코드 스타일 가이드 작성
- [ ] 커밋 컨벤션 정의
- [ ] 이슈 템플릿 작성
- [ ] README.md 초안 작성
- [ ] LICENSE 파일 추가
- [ ] CONTRIBUTING.md 작성 (오픈소스인 경우)

**기록 및 문서화:**
- [ ] 프로젝트 결정 사항 문서화
- [ ] 아키텍처 결정 기록 (ADR - Architecture Decision Records)
- [ ] 변경 로그 (CHANGELOG.md) 시작

---

## 1. 전체 방향 정의 및 아키텍처 설계

### 1.1 프로젝트 목표 및 요구사항

**핵심 목표:**
- Rust로 작성된 독립적인 저전력 OS 커널 개발
- 리눅스 커널과 완전히 독립적인 구현
- 노트북 환경에 최적화된 전력 관리
- 메모리 안전성과 성능의 균형

**비기능적 요구사항:**
- 부팅 시간: 5초 이내
- 유휴 상태 전력 소비: 최소화 (목표: 5W 이하)
- 메모리 사용량: 최소 64MB RAM에서 동작
- 안정성: 커널 패닉 없는 장시간 운영 목표

### 1.2 아키텍처 설계

**전체 시스템 구조:**
```
┌─────────────────────────────────────┐
│         Application Layer            │
│   (Shell, GUI, User Programs)       │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│         System Call Interface        │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│         Kernel Core                  │
│  ┌──────────┐  ┌──────────┐        │
│  │Scheduler │  │  Memory  │        │
│  └──────────┘  └──────────┘        │
│  ┌──────────┐  ┌──────────┐        │
│  │  Power   │  │  Driver  │        │
│  │  Mgmt    │  │  Manager │        │
│  └──────────┘  └──────────┘        │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│         Hardware Abstraction         │
│   (Interrupts, Timers, MMIO)        │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│         Hardware Layer               │
│   (CPU, Memory, I/O Devices)        │
└─────────────────────────────────────┘
```

**커널 모듈 구조:**
- `kernel/`
  - `boot/` - 부트로더 인터페이스 및 초기화
  - `memory/` - 메모리 관리 (물리/가상, 할당자)
  - `scheduler/` - 프로세스/스레드 스케줄링
  - `power/` - 전력 관리 및 ACPI
  - `drivers/` - 하드웨어 드라이버
  - `interrupts/` - 인터럽트 핸들러
  - `sync/` - 동기화 프리미티브 (Mutex, Spinlock 등)
  - `syscall/` - 시스템 콜 인터페이스
  - `fs/` - 파일시스템 인터페이스
  - `net/` - 네트워크 스택

### 1.3 기술 스택 결정

**언어 및 런타임:**
- Rust (nightly) - 메모리 안전성 보장
- `no_std` 환경 - 표준 라이브러리 제외
- `#![no_main]` - 커스텀 엔트리 포인트

**필수 크레이트:**
- `bootloader` - 부트로더 통합
- `x86_64` - x86_64 아키텍처 지원
- `volatile` - 하드웨어 레지스터 접근
- `spin` - 스핀락 구현
- `linked_list_allocator` - 힙 할당자 (초기 단계)
- `uart_16550` - 시리얼 포트 통신

**선택적 크레이트 (단계별 도입):**
- `acpi` - ACPI 테이블 파싱
- `pci` - PCI 디바이스 스캔
- `embedded-graphics` - GUI 프레임워크
- `smoltcp` - 네트워크 스택

### 1.4 타겟 하드웨어

**지원 아키텍처:**
- x86_64 (64비트)
- UEFI 펌웨어 (우선)
- BIOS 레거시 부팅 (선택적)

**하드웨어 요구사항:**
- CPU: Intel Core 시리즈 또는 AMD Ryzen (x86_64)
- RAM: 최소 64MB (권장: 512MB 이상)
- 저장장치: SSD/HDD (ATA/SATA)
- 디스플레이: VGA 텍스트 모드 또는 VESA 프레임버퍼

---

## 2. Rust OS 개발 환경 구축

### 2.1 Rust 툴체인 설치 및 설정

**필수 컴포넌트 설치:**
```bash
# Nightly Rust 설치
rustup install nightly
rustup default nightly

# 필수 컴포넌트 추가
rustup component add rust-src llvm-tools-preview

# x86_64 타겟 추가
rustup target add x86_64-unknown-none

# 부트이미지 생성 도구 설치
cargo install bootimage --version "^0.11.0"
```

**검증:**
```bash
rustc --version --verbose
rustup show
```

### 2.2 크로스 컴파일 환경 설정

**필수 도구 설치 (Windows):**
- LLVM (clang 포함)
- NASM (어셈블러)
- 빌드 도구 (Visual Studio Build Tools)

**필수 도구 설치 (Linux):**
```bash
sudo apt-get update
sudo apt-get install \
    build-essential \
    nasm \
    qemu-system-x86 \
    qemu-utils \
    ovmf
```

**Cargo 설정 (`~/.cargo/config.toml` 또는 `./.cargo/config.toml`):**
```toml
[build]
target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
runner = "bootimage runner"
```

### 2.3 프로젝트 초기 구조 생성

**Cargo.toml 설정:**
```toml
[package]
name = "simple_os"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[dependencies]
bootloader = { version = "0.11", features = ["map_physical_memory"] }
x86_64 = "0.14"
volatile = "0.4"
spin = "0.9"
uart_16550 = "0.2"

[profile.dev]
panic = "abort"
opt-level = 0

[profile.release]
panic = "abort"
opt-level = 3
lto = true
codegen-units = 1
```

**프로젝트 디렉토리 구조:**
```
simple_os/
├── Cargo.toml
├── Cargo.lock
├── .cargo/
│   └── config.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── boot/
│   ├── memory/
│   ├── scheduler/
│   ├── power/
│   ├── drivers/
│   ├── interrupts/
│   └── sync/
├── tests/
└── docs/
```

### 2.4 디버깅 환경 설정

**GDB 설정:**
```bash
# GDB 설치 (Linux)
sudo apt-get install gdb

# QEMU GDB 서버 시작
qemu-system-x86_64 -s -S -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin

# 별도 터미널에서 GDB 연결
rust-gdb target/x86_64-unknown-none/debug/simple_os
(gdb) target remote :1234
```

**로깅 시스템:**
- 시리얼 포트 기반 로깅 구현
- `log` 크레이트 사용 (환경에 맞게 어댑터 구현)
- 레벨별 로그 필터링 (ERROR, WARN, INFO, DEBUG, TRACE)

### 2.5 테스트 환경 구축

**QEMU 실행 스크립트 (`run.sh` 또는 `run.bat`):**
```bash
#!/bin/bash
# Linux용
cargo bootimage
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio \
    -display none
```

**Continuous Integration 설정:**
- GitHub Actions 또는 GitLab CI
- 단위 테스트 및 통합 테스트 자동화
- 커널 빌드 검증

---

## 3. 부트로더 및 커널 초기화

### 3.1 부트로더 통합

**bootloader 크레이트 사용:**
```toml
[dependencies]
bootloader = { version = "0.11", features = ["map_physical_memory"] }
```

**부트 프로토콜:**
- Multiboot2 또는 bootloader 크레이트 사용
- 부트 정보 구조체 전달 (메모리 맵, ACPI RSDP 등)

### 3.2 커널 엔트리 포인트

**`src/main.rs` 기본 구조:**
```rust
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 초기화 순서:
    // 1. 인터럽트 디스크립터 테이블 (IDT) 설정
    // 2. 메모리 관리자 초기화
    // 3. 스케줄러 초기화
    // 4. 전력 관리 초기화
    // 5. 드라이버 초기화
    // 6. 쉘/GUI 시작
    
    kernel_init();
    
    loop {
        x86_64::instructions::hlt();
    }
}

fn kernel_init() {
    // 초기화 코드
}
```

### 3.3 인터럽트 설정

**IDT (Interrupt Descriptor Table) 구현:**
- 예외 핸들러: Divide Error, Page Fault, General Protection 등
- 하드웨어 인터럽트: Timer, Keyboard, Serial 등
- 시스템 콜 인터럽트 (INT 0x80 또는 SYSCALL)

**초기화 순서:**
1. IDT 구조체 생성 및 로드
2. PIC (Programmable Interrupt Controller) 리매핑
3. 인터럽트 활성화 (`sti`)

### 3.4 초기 메모리 설정

**메모리 맵 파싱:**
- 부트로더에서 전달된 메모리 맵 읽기
- 사용 가능한 메모리 영역 식별
- 커널 코드/데이터 영역 보호

**가상 메모리 초기화:**
- 페이지 테이블 구조 생성
- 커널 매핑 설정
- 힙 영역 할당

---

## 4. 커널 코어 구성

### 4.1 메모리 관리 시스템

**물리 메모리 관리:**
- 프레임 할당자 구현
  - 비트맵 기반 할당자 (초기)
  - Buddy allocator (고급)
- 4KB 페이지 단위 관리
- DMA 영역 보호

**가상 메모리 관리:**
- 4단계 페이지 테이블 (x86_64)
- 페이지 디렉토리 구조
- TLB 관리 및 플러시

**힙 할당자:**
- 초기: `linked_list_allocator` 사용
- 후기: 커스텀 allocator 구현
  - Buddy allocator
  - Slab allocator (커널 객체용)

**메모리 보안:**
- 페이지 권한 설정 (Read/Write/Execute)
- ASLR (Address Space Layout Randomization) 지원
- 스택 오버플로우 보호

### 4.2 프로세스 및 스레드 스케줄러

**프로세스 구조:**
```rust
pub struct Process {
    pub pid: u64,
    pub state: ProcessState,
    pub memory_space: MemorySpace,
    pub registers: Registers,
    pub priority: u8,
    pub power_policy: PowerPolicy,
}

pub enum ProcessState {
    Running,
    Ready,
    Blocked,
    Sleeping,
    Zombie,
}
```

**스케줄링 알고리즘:**
- 초기: Round-Robin 스케줄러
- 후기: CFS (Completely Fair Scheduler) 스타일
- 전력 효율 고려:
  - CPU 집약적 작업은 낮은 클럭에서도 실행
  - I/O 대기 프로세스는 즉시 깨우기

**컨텍스트 스위칭:**
- 레지스터 저장/복원
- 페이지 테이블 전환
- TLB 플러시

**동기화 프리미티브:**
- Spinlock (짧은 대기용)
- Mutex (스레드 블로킹)
- Semaphore
- Condition Variable
- Reader-Writer Lock

### 4.3 시스템 콜 인터페이스

**시스템 콜 번호 정의:**
- `SYS_EXIT` - 프로세스 종료
- `SYS_FORK` - 프로세스 복제
- `SYS_READ` / `SYS_WRITE` - I/O
- `SYS_OPEN` / `SYS_CLOSE` - 파일
- `SYS_BRK` - 힙 확장
- `SYS_SLEEP` - 대기
- `SYS_POWER` - 전력 관리

**시스템 콜 핸들러:**
- 인터럽트 기반 또는 SYSCALL 명령어 사용
- 파라미터 검증 및 보안 체크
- 커널/유저 모드 전환

---

## 5. 전력 관리 시스템

### 5.1 CPU 전력 관리

**P-State (Performance State) 제어:**
- MSR (Model Specific Register) 접근
- `IA32_PERF_CTL` - 클럭 제어
- 동적 클럭 스케일링 (DVFS)

**C-State (Idle State) 관리:**
- C0: 활성 상태
- C1: Halt (hlt 명령어)
- C1E: Enhanced Halt
- C3: Deep Sleep
- C6: Deeper Sleep

**구현 전략:**
```rust
pub struct PowerManager {
    current_p_state: PState,
    current_c_state: CState,
    acpi_tables: AcpiTables,
}

impl PowerManager {
    pub fn set_cpu_frequency(&mut self, freq_mhz: u32) {
        // MSR을 통한 클럭 설정
    }
    
    pub fn enter_idle(&mut self) {
        // 가장 깊은 가능한 C-State 진입
        unsafe {
            x86_64::instructions::hlt();
        }
    }
}
```

### 5.2 ACPI (Advanced Configuration and Power Interface) 파싱

**필수 ACPI 테이블:**
- RSDP (Root System Description Pointer)
- RSDT/XSDT (Root System Description Table)
- FADT (Fixed ACPI Description Table)
- DSDT (Differentiated System Description Table)
- SSDT (Secondary System Description Table)

**배터리 관리:**
- `_BST` (Battery Status) 메서드 호출
- `_BIF` (Battery Information) 파싱
- 배터리 잔량 모니터링

**전원 상태:**
- S0: 완전 활성
- S1-S3: Sleep 상태
- S4: Hibernate
- S5: 완전 종료

**ACPI 이벤트 처리:**
- 전원 버튼 이벤트
- 리드 스위치 (노트북 덮개)
- 배터리 저전력 경고

### 5.3 동적 전력 스케일링

**로드 기반 스케일링:**
- CPU 사용률 모니터링
- 부하가 낮으면 클럭 다운
- 부하가 높으면 클럭 업

**이벤트 기반 최적화:**
- I/O 대기 시 즉시 클럭 다운
- 타이머 인터럽트에서 로드 재평가
- 스케줄러와 협력하여 전력 최적화

**사용자 정책:**
- 성능 우선 모드
- 균형 모드
- 전력 절약 모드

### 5.4 주변 장치 전력 관리

**PCIe ASPM (Active State Power Management):**
- L0s, L1 상태 지원
- 링크 전력 관리

**USB 자동 Suspend:**
- 미사용 USB 장치 자동 suspend
- 이벤트 발생 시 resume

**디스플레이 백라이트 제어:**
- 밝기 조절
- 자동 꺼짐 (idle 후)

---

## 6. 하드웨어 드라이버 구현

### 6.1 시리얼 포트 (UART)

**uart_16550 크레이트 사용:**
```rust
use uart_16550::SerialPort;

pub fn init_serial() {
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();
    // 로깅 시스템과 연결
}
```

**기능:**
- 시리얼 출력 (디버깅/로깅)
- 시리얼 입력 (선택적)
- 인터럽트 기반 I/O

### 6.2 키보드 드라이버

**PS/2 키보드:**
- 스캔코드 수신 및 파싱
- 키 맵핑 (스캔코드 → 문자)
- 특수 키 처리 (Ctrl, Alt, Shift)
- 키보드 인터럽트 핸들러

**구현:**
```rust
pub struct Keyboard {
    key_buffer: RingBuffer<KeyEvent>,
    modifier_state: ModifierState,
}

pub enum KeyEvent {
    Pressed(KeyCode),
    Released(KeyCode),
}
```

### 6.3 VGA 텍스트 모드

**vga_buffer 크레이트 또는 직접 구현:**
- 80x25 텍스트 모드
- 색상 지원 (16색)
- 커서 제어
- 스크롤링

**후기 프레임버퍼 모드:**
- VESA 프레임버퍼 초기화
- 픽셀 직접 그리기
- Double buffering

### 6.4 타이머 드라이버

**PIT (Programmable Interval Timer):**
- 1ms 타이커 설정
- 스케줄러 타임슬라이스
- 시스템 시간 유지

**HPET (High Precision Event Timer):**
- 더 정밀한 타이밍
- ACPI에서 감지 및 초기화

**RTC (Real-Time Clock):**
- 실제 시간 읽기/쓰기
- CMOS 레지스터 접근

### 6.5 저장장치 드라이버

**ATA/SATA 컨트롤러:**
- PIO (Programmed I/O) 모드
- DMA 모드 (후기)
- 파티션 테이블 파싱

**파일시스템 드라이버:**
- FAT32 읽기/쓰기
- 디렉토리 탐색
- 파일 메타데이터

### 6.6 네트워크 드라이버

**이더넷 컨트롤러:**
- PCI 디바이스 스캔
- MAC 주소 읽기
- 패킷 송수신

**네트워크 스택 (smoltcp):**
- IP, TCP, UDP 프로토콜
- ARP, ICMP 지원
- 소켓 인터페이스

### 6.7 드라이버 아키텍처

**드라이버 인터페이스:**
```rust
pub trait Driver {
    fn name(&self) -> &str;
    fn init(&mut self) -> Result<(), DriverError>;
    fn interrupt_handler(&mut self) -> Option<InterruptResult>;
}
```

**드라이버 매니저:**
- 드라이버 등록/해제
- 인터럽트 라우팅
- 전력 관리 협력

---

## 7. 빌드 시스템 및 부팅 테스트

### 7.1 빌드 프로세스

**개발 빌드:**
```bash
cargo build
cargo bootimage
```

**릴리즈 빌드:**
```bash
cargo build --release
cargo bootimage --release
```

**빌드 최적화:**
- LTO (Link Time Optimization)
- 코드 크기 최적화 (`opt-level = "z"` 또는 `"s"`)
- 디버그 심볼 제거 (릴리즈)

### 7.2 QEMU 테스트

**기본 실행:**
```bash
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio \
    -display none
```

**고급 옵션:**
```bash
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio \
    -monitor stdio \
    -m 512M \
    -cpu host \
    -smp 2 \
    -machine q35 \
    -s -S  # GDB 서버
```

**UEFI 펌웨어 사용:**
```bash
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -bios /usr/share/ovmf/OVMF.fd \
    -serial stdio
```

### 7.3 실제 하드웨어 테스트

**USB 부팅 이미지 생성:**
```bash
# Linux
dd if=target/x86_64-unknown-none/debug/bootimage-simple_os.bin of=/dev/sdX bs=4M status=progress

# Windows
# Rufus 또는 Win32DiskImager 사용
```

**주의사항:**
- 테스트 전 데이터 백업
- 부팅 순서 변경 필요
- 커널 패닉 시 복구 방법 준비

### 7.4 단위 테스트 및 통합 테스트

**no_std 테스트 프레임워크:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test_case]
    fn test_memory_alloc() {
        // 테스트 코드
    }
}
```

**통합 테스트:**
- 커널 기능 통합 테스트
- 드라이버 통합 테스트
- 전력 관리 통합 테스트

---

## 8. Shell 및 GUI 시스템

### 8.1 CLI Shell 구현

**기본 기능:**
- 명령어 파싱 및 실행
- 파이프 (`|`) 지원
- 리다이렉션 (`>`, `<`) 지원
- 환경 변수
- 히스토리 (방향키)

**구현 구조:**
```rust
pub struct Shell {
    prompt: String,
    history: Vec<String>,
    env: HashMap<String, String>,
}

impl Shell {
    pub fn run(&mut self) {
        loop {
            let input = self.read_line();
            let command = self.parse(input);
            self.execute(command);
        }
    }
}
```

**기본 명령어:**
- `help` - 도움말
- `ls` - 디렉토리 목록
- `cd` - 디렉토리 변경
- `cat` - 파일 내용 출력
- `echo` - 텍스트 출력
- `power` - 전력 상태/설정
- `ps` - 프로세스 목록
- `kill` - 프로세스 종료

### 8.2 GUI 시스템

**프레임버퍼 기반 GUI:**
- VESA 프레임버퍼 초기화
- Double buffering
- 창 관리자 (기본)

**embedded-graphics 사용:**
```rust
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::*,
};
```

**기본 위젯:**
- 버튼
- 텍스트 입력
- 메뉴
- 대화상자

**윈도우 시스템:**
- 다중 윈도우 지원
- 창 이동/리사이즈
- 포커스 관리

### 8.3 텍스트 에디터

**기본 기능:**
- 파일 열기/저장
- 텍스트 편집
- 구문 강조 (선택적)

### 8.4 파일 관리자

**기능:**
- 디렉토리 탐색
- 파일 복사/이동/삭제
- 파일 속성 표시

---

## 9. 고급 기능 및 최적화

### 9.1 고급 전력 관리

**Dynamic Voltage and Frequency Scaling (DVFS):**
- 부하에 따른 실시간 클럭 조절
- 전압 조절 (가능한 경우)

**Sleep/Resume 지원:**
- S3 (Suspend to RAM) 구현
- 상태 저장 및 복원
- 하드웨어 재초기화

**Thermal Management:**
- CPU 온도 모니터링
- 쓰로틀링 (과열 방지)
- 팬 제어

### 9.2 고급 메모리 관리

**Swap 지원:**
- 디스크 스왑 영역
- 페이지 아웃/인
- 스왑 통계

**메모리 압축:**
- ZRAM 스타일 압축
- 메모리 효율성 향상

**NUMA 지원:**
- 멀티 소켓 시스템
- 노드별 메모리 할당

### 9.3 파일시스템

**FAT32 완전 구현:**
- 읽기/쓰기
- 디렉토리 생성/삭제
- 파일 속성

**Ext2 지원 (선택적):**
- 리눅스 호환성
- 저널링 없는 단순 구현

**가상 파일시스템 (VFS):**
- 통합 파일시스템 인터페이스
- `/proc`, `/sys` 가상 파일시스템

### 9.4 프로세스 관리 고급 기능

**프로세스 분리:**
- 페이지 테이블 분리
- 메모리 보호
- 시스템 콜 격리

**시그널 시스템:**
- 프로세스 간 통신
- 시그널 핸들러

**IPC (Inter-Process Communication):**
- 파이프
- 공유 메모리
- 메시지 큐

### 9.5 네트워킹

**TCP/IP 스택 (smoltcp):**
- IP 라우팅
- TCP 연결 관리
- UDP 소켓

**네트워크 서비스:**
- HTTP 서버 (기본)
- SSH 서버 (선택적)
- DNS 클라이언트

### 9.6 보안 기능

**커널 보안:**
- 스택 카나리
- 페이지 권한 강화
- 시스템 콜 필터링

**사용자 인증:**
- 로그인 시스템
- 권한 관리

### 9.7 성능 최적화

**커널 최적화:**
- 인라인 어셈블리 (핫패스)
- CPU 특정 최적화
- 캐시 친화적 데이터 구조

**I/O 최적화:**
- 비동기 I/O
- I/O 스케줄러
- DMA 활용

**컴파일러 최적화:**
- LTO
- Profile-Guided Optimization (PGO)
- 코드 크기 최적화

### 9.8 멀티코어 지원

**SMP (Symmetric Multiprocessing):**
- AP (Application Processor) 초기화
- CPU 간 통신
- 로드 밸런싱

**락 최적화:**
- NUMA 인식 락
- Lock-free 알고리즘 (가능한 경우)

---

## 10. 참고 OS 프로젝트 및 학습 자료

### 10.1 참고 OS 프로젝트

**Redox OS:**
- 특징: Rust로 완전히 작성된 Unix-like OS
- 학습 포인트: 전체 시스템 구조, 파일시스템, 네트워킹
- 레포지토리: https://github.com/redox-os/redox
- 문서: https://doc.redox-os.org/

**Theseus OS:**
- 특징: 모듈형 런타임 시스템, 컴파일 타임 리소스 관리
- 학습 포인트: Rust의 타입 시스템 활용, 모듈 아키텍처
- 레포지토리: https://github.com/theseus-os/Theseus
- 논문: Theseus: an Experiment in Operating System Structure and State Management

**Tock OS:**
- 특징: 임베디드 시스템용, 초저전력 최적화
- 학습 포인트: 전력 관리, 임베디드 시스템 설계
- 레포지토리: https://github.com/tock/tock
- 문서: https://book.tockos.org/

**IntermezzOS:**
- 특징: 학습용 미니멀 OS
- 학습 포인트: 기본 커널 구조, 부트로더 통합
- 웹사이트: https://intermezzos.github.io/

**RuxOS:**
- 특징: 모듈형 교육용 OS
- 학습 포인트: 각 모듈의 독립적 학습
- 레포지토리: https://github.com/sysprog21/rux

### 10.2 필수 학습 자료

**Rust OS 개발:**
- "Writing an OS in Rust" (Philipp Oppermann)
  - 웹사이트: https://os.phil-opp.com/
  - 부트로더, 메모리 관리, 인터럽트 등 단계별 가이드

- "The Embedded Rust Book"
  - 웹사이트: https://docs.rust-embedded.org/book/
  - no_std Rust 프로그래밍

**운영체제 이론:**
- "Operating Systems: Three Easy Pieces" (OSTEP)
  - 웹사이트: http://pages.cs.wisc.edu/~remzi/OSTEP/
  - 가상화, 동시성, 영속성

- "Modern Operating Systems" (Andrew Tanenbaum)
  - 전통적인 OS 교과서

**하드웨어 참조:**
- Intel 64 and IA-32 Architectures Software Developer's Manual
  - CPU 아키텍처, 명령어 세트, 시스템 프로그래밍

- ACPI Specification
  - 전력 관리 인터페이스

- UEFI Specification
  - 부트 프로세스

### 10.3 유용한 도구 및 리소스

**디버깅 도구:**
- GDB (GNU Debugger)
- QEMU Monitor
- Serial 로깅

**문서화:**
- Rust 문서 시스템 (`cargo doc`)
- 커널 내부 문서화
- 사용자 매뉴얼

**커뮤니티:**
- OSDev.org 포럼
- Reddit r/osdev
- Rust OS 개발자 커뮤니티

---

## 개발 체크리스트

### 초기 단계 (1-3개월)
- [ ] Rust 개발 환경 구축
- [ ] 기본 커널 부팅 성공
- [ ] 인터럽트 핸들러 구현
- [ ] 메모리 관리 기본 구현
- [ ] 시리얼 포트 출력
- [ ] VGA 텍스트 모드 출력

### 중기 단계 (3-6개월)
- [ ] 스케줄러 구현
- [ ] 키보드 입력 처리
- [ ] 기본 드라이버 (타이머, 키보드, VGA)
- [ ] 힙 할당자 구현
- [ ] 시스템 콜 인터페이스
- [ ] 기본 Shell 구현

### 후기 단계 (6-12개월)
- [ ] 전력 관리 시스템 (ACPI 파싱)
- [ ] 동적 전력 스케일링
- [ ] 파일시스템 (FAT32)
- [ ] 네트워크 드라이버 및 스택
- [ ] GUI 시스템 (기본)
- [ ] 멀티코어 지원

### 고급 단계 (12개월+)
- [ ] Sleep/Resume 지원
- [ ] 보안 기능 강화
- [ ] 성능 최적화
- [ ] 추가 파일시스템 지원
- [ ] 네트워크 서비스
- [ ] 문서화 및 테스트

---

## 주의사항 및 고려사항

### 하드웨어 호환성
- 다양한 노트북 모델에서 테스트 필요
- ACPI 구현 차이 고려
- 드라이버 호환성 문제 해결

### 보안
- 커널은 모든 권한을 가지므로 버그가 치명적
- 메모리 안전성 검증 중요
- 시스템 콜 검증 필수

### 성능
- 초기 구현은 안정성 우선
- 프로파일링 후 최적화
- 메모리 사용량 모니터링

### 문서화
- 코드 주석 상세히 작성
- 아키텍처 문서 유지
- API 문서 자동 생성

---

## 결론

이 로드맵은 Rust 기반 초저전력 노트북 OS 개발을 위한 포괄적인 가이드입니다. 각 단계는 점진적으로 복잡도를 증가시키며, 실제 하드웨어에서 동작하는 완전한 OS를 목표로 합니다. 

핵심은 **안정성**, **전력 효율**, **확장 가능성**입니다. 각 기능을 구현할 때 이 세 가지 원칙을 고려하여 설계하고 구현해야 합니다.

개발 과정에서 문제가 발생하면 참고 OS 프로젝트와 커뮤니티를 적극 활용하고, 단계별로 테스트하며 진행하는 것이 중요합니다.
