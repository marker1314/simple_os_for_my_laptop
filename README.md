# Simple OS for My Laptop

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-x86__64-lightgrey.svg)](https://en.wikipedia.org/wiki/X86-64)

Rust로 작성된 초저전력 노트북 운영체제 커널 프로젝트입니다. 리눅스 커널과 완전히 독립적인 구현으로, 노트북 환경에 최적화된 전력 관리를 목표로 합니다.

## 📋 목차

- [특징](#-특징)
- [목표](#-목표)
- [시작하기](#-시작하기)
- [요구사항](#-요구사항)
- [빌드 및 실행](#-빌드-및-실행)
- [프로젝트 구조](#-프로젝트-구조)
- [개발 로드맵](#-개발-로드맵)
- [기여하기](#-기여하기)
- [라이선스](#-라이선스)
- [참고 자료](#-참고-자료)

## ✨ 특징

- **메모리 안전성**: Rust의 타입 시스템과 메모리 안전성 보장으로 커널 레벨 버그 최소화
- **초저전력 설계**: ACPI 기반 전력 관리 및 동적 CPU 클럭 스케일링
- **독립적 구현**: 리눅스 커널과 완전히 독립적인 커널 설계
- **모듈형 아키텍처**: 확장 가능하고 유지보수하기 쉬운 구조
- **no_std 환경**: 표준 라이브러리 없이 실행되는 경량 커널

## 🎯 목표

### 기능적 목표
- 독립적인 운영체제 커널 구현
- 기본 드라이버 지원 (키보드, 디스플레이, 저장장치, 네트워크)
- 전력 관리 시스템 (ACPI 파싱, 동적 스케일링)
- 기본 Shell 및 GUI 시스템
- 파일시스템 지원 (FAT32)

### 비기능적 목표
- **부팅 시간**: 5초 이내
- **유휴 전력 소비**: 5W 이하
- **메모리 사용량**: 최소 64MB RAM에서 동작 (권장: 512MB 이상)
- **안정성**: 커널 패닉 없는 장시간 운영

## 🚀 시작하기

### 요구사항

#### 필수 도구
- **Rust (nightly)**: `rustup install nightly`
- **bootimage**: `cargo install bootimage`
- **QEMU**: 가상화 환경에서 테스트하기 위함

#### Windows
```powershell
# Rust 설치
winget install Rustlang.Rustup
rustup install nightly
rustup default nightly

# 필수 컴포넌트
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none

# bootimage 설치
cargo install bootimage

# QEMU 설치 (선택사항)
winget install SoftwareFreedomConservancy.QEMU
```

#### Linux
```bash
# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install nightly
rustup default nightly

# 필수 컴포넌트
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none

# bootimage 설치
cargo install bootimage

# QEMU 및 기타 도구 설치
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    nasm \
    qemu-system-x86 \
    qemu-utils \
    ovmf
```

### 빌드 및 실행

#### 1. 저장소 클론
```bash
git clone https://github.com/yourusername/simple_os_for_my_laptop.git
cd simple_os_for_my_laptop
```

#### 2. 커널 빌드
```bash
# 디버그 빌드
cargo build

# 부팅 이미지 생성
cargo bootimage

# 릴리즈 빌드 (최적화)
cargo build --release
cargo bootimage --release
```

#### 3. QEMU에서 실행
```bash
# 기본 실행
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio \
    -display none

# 또는 스크립트 사용 (Linux/macOS)
./run.sh

# Windows에서는
.\run.bat
```

#### 4. 디버깅 모드
```bash
# QEMU를 GDB 서버 모드로 실행
qemu-system-x86_64 \
    -s -S \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio

# 별도 터미널에서 GDB 연결
rust-gdb target/x86_64-unknown-none/debug/simple_os
(gdb) target remote :1234
```

## 📁 프로젝트 구조

```
simple_os_for_my_laptop/
├── Cargo.toml              # 프로젝트 설정 및 의존성
├── Cargo.lock              # 의존성 버전 고정
├── README.md               # 프로젝트 소개 (이 파일)
├── roadmap.md              # 상세 개발 로드맵
├── LICENSE                 # 라이선스 파일
├── .cargo/
│   └── config.toml         # Cargo 설정
├── src/
│   ├── main.rs            # 커널 엔트리 포인트
│   ├── lib.rs             # 라이브러리 루트
│   ├── boot/              # 부트로더 인터페이스
│   ├── memory/            # 메모리 관리
│   │   ├── allocator.rs   # 힙 할당자
│   │   ├── paging.rs      # 가상 메모리
│   │   └── frame.rs       # 물리 메모리 프레임
│   ├── scheduler/         # 프로세스/스레드 스케줄러
│   ├── power/             # 전력 관리
│   │   ├── acpi.rs        # ACPI 파싱
│   │   └── scaling.rs     # 동적 스케일링
│   ├── drivers/           # 하드웨어 드라이버
│   │   ├── keyboard.rs    # 키보드 드라이버
│   │   ├── vga.rs         # VGA 디스플레이
│   │   ├── timer.rs       # 타이머
│   │   └── serial.rs      # 시리얼 포트
│   ├── interrupts/        # 인터럽트 핸들러
│   ├── sync/              # 동기화 프리미티브
│   ├── syscall/           # 시스템 콜 인터페이스
│   ├── fs/                # 파일시스템 인터페이스
│   └── net/               # 네트워크 스택
├── tests/                 # 통합 테스트
├── docs/                  # 추가 문서
└── scripts/               # 빌드/실행 스크립트
    ├── run.sh             # Linux/macOS 실행 스크립트
    └── run.bat            # Windows 실행 스크립트
```

## 🗺️ 개발 로드맵

상세한 개발 로드맵은 [roadmap.md](roadmap.md)를 참고하세요.

### 현재 상태

**1단계: 전체 방향 정의 및 아키텍처 설계 (완료)**
- [x] 프로젝트 구조 설계
- [x] 아키텍처 문서 작성 (`docs/architecture.md`)
- [x] 프로젝트 요구사항 문서화 (`docs/requirements.md`)
- [x] 기본 프로젝트 구조 생성
- [x] Cargo.toml 및 설정 파일 생성
- [x] 커널 모듈 구조 생성

**2단계: Rust OS 개발 환경 구축 (진행 예정)**
- [ ] Rust 툴체인 설치 및 설정
- [ ] 크로스 컴파일 환경 설정
- [ ] 디버깅 환경 설정
- [ ] 테스트 환경 구축

**초기 단계 (예정)**
- [ ] 기본 커널 부팅
- [ ] 인터럽트 핸들러 구현
- [ ] 메모리 관리 기본 구현

### 계획된 기능

**중기 목표**
- [ ] 스케줄러 구현
- [ ] 기본 드라이버 (키보드, VGA, 타이머)
- [ ] 시스템 콜 인터페이스
- [ ] 기본 Shell 구현

**장기 목표**
- [ ] 전력 관리 시스템 (ACPI 완전 파싱)
- [ ] 동적 전력 스케일링
- [ ] 파일시스템 (FAT32)
- [ ] 네트워크 드라이버 및 스택
- [ ] GUI 시스템
- [ ] 멀티코어 지원

## 🛠️ 기술 스택

### 핵심 기술
- **언어**: Rust (nightly)
- **아키텍처**: x86_64
- **부트 프로토콜**: UEFI (BIOS 레거시 지원 예정)
- **환경**: `no_std` (표준 라이브러리 없음)

### 주요 크레이트
- `bootloader` - 부트로더 통합
- `x86_64` - x86_64 아키텍처 지원
- `volatile` - 하드웨어 레지스터 접근
- `spin` - 스핀락 구현
- `uart_16550` - 시리얼 포트 통신

### 향후 추가 예정
- `acpi` - ACPI 테이블 파싱
- `pci` - PCI 디바이스 스캔
- `embedded-graphics` - GUI 프레임워크
- `smoltcp` - 네트워크 스택

## 🤝 기여하기

기여를 환영합니다! 프로젝트에 기여하고 싶으시다면:

1. 이 저장소를 포크하세요
2. 기능 브랜치를 생성하세요 (`git checkout -b feature/amazing-feature`)
3. 변경사항을 커밋하세요 (`git commit -m 'Add some amazing feature'`)
4. 브랜치에 푸시하세요 (`git push origin feature/amazing-feature`)
5. Pull Request를 열어주세요

### 코드 스타일
- `rustfmt`를 사용하여 코드 포맷팅
- `clippy` 경고 해결
- 의미 있는 커밋 메시지 작성

### 이슈 리포트
버그를 발견하셨거나 기능 제안이 있으시면 [Issues](https://github.com/yourusername/simple_os_for_my_laptop/issues)에 등록해주세요.

## 📝 라이선스

이 프로젝트의 라이선스는 아직 결정되지 않았습니다. 프로젝트 정책 결정 후 LICENSE 파일이 추가될 예정입니다.

## 📚 참고 자료

### 학습 자료
- [Writing an OS in Rust](https://os.phil-opp.com/) - Rust OS 개발 튜토리얼
- [The Embedded Rust Book](https://docs.rust-embedded.org/book/) - no_std Rust 프로그래밍
- [Operating Systems: Three Easy Pieces](http://pages.cs.wisc.edu/~remzi/OSTEP/) - 운영체제 이론

### 참고 OS 프로젝트
- [Redox OS](https://github.com/redox-os/redox) - Rust로 작성된 Unix-like OS
- [Theseus OS](https://github.com/theseus-os/Theseus) - 모듈형 런타임 시스템
- [Tock OS](https://github.com/tock/tock) - 임베디드 시스템용 초저전력 OS
- [IntermezzOS](https://intermezzos.github.io/) - 학습용 미니멀 OS

### 하드웨어 참조
- [Intel 64 and IA-32 Architectures Software Developer's Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [ACPI Specification](https://uefi.org/specifications)
- [UEFI Specification](https://uefi.org/specifications)

## ⚠️ 주의사항

이 프로젝트는 **실험적**이며 개발 중입니다.

- 프로덕션 환경에서 사용하지 마세요
- 데이터 손실 위험이 있으므로 중요한 데이터가 있는 시스템에서는 테스트하지 마세요
- 실제 하드웨어에서 테스트할 때는 전용 테스트 머신을 사용하세요
- 커널 레벨 버그는 시스템을 완전히 멈출 수 있습니다

## 📧 연락처

프로젝트 관련 문의사항이 있으시면 [Issues](https://github.com/yourusername/simple_os_for_my_laptop/issues)를 통해 연락해주세요.

---

**Made with ❤️ and Rust**

