# 개발 환경 설정 가이드

이 문서는 Simple OS 개발을 위한 개발 환경 설정 방법을 설명합니다.

## 목차

1. [필수 요구사항](#필수-요구사항)
2. [Windows 환경 설정](#windows-환경-설정)
3. [Linux 환경 설정](#linux-환경-설정)
4. [설정 검증](#설정-검증)
5. [문제 해결](#문제-해결)

## 필수 요구사항

### 필수 도구

- **Rust (nightly)**: 커널 개발에 필요
- **bootimage**: 부팅 이미지 생성 도구
- **QEMU**: 가상화 환경에서 테스트
- **LLVM/clang**: 크로스 컴파일 (Windows의 경우)

### 선택적 도구

- **GDB**: 디버깅 (Linux 권장)
- **NASM**: 어셈블러 (필요한 경우)

## Windows 환경 설정

### 1. Rust 설치

Rust가 설치되어 있지 않다면:
```powershell
# Rust 설치
winget install Rustlang.Rustup

# 또는 공식 사이트에서 다운로드
# https://www.rust-lang.org/tools/install
```

### 2. 자동 설정 스크립트 실행

프로젝트 루트에서 실행:
```powershell
.\scripts\setup.ps1
```

이 스크립트는 다음을 자동으로 수행합니다:
- Rust nightly 설치
- 필수 컴포넌트 추가 (rust-src, llvm-tools-preview)
- x86_64-unknown-none 타겟 추가
- bootimage 설치

### 3. 수동 설정 (스크립트를 사용하지 않는 경우)

```powershell
# Nightly Rust 설치
rustup install nightly
rustup default nightly

# 필수 컴포넌트 추가
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly

# x86_64 타겟 추가
rustup target add x86_64-unknown-none --toolchain nightly

# bootimage 설치
cargo install bootimage --version "^0.11.0"
```

### 4. QEMU 설치

```powershell
# winget을 사용한 설치
winget install SoftwareFreedomConservancy.QEMU

# 또는 공식 사이트에서 다운로드
# https://www.qemu.org/download/#windows
```

### 5. LLVM/clang 설치 (선택적, 권장)

```powershell
# LLVM 설치
winget install LLVM.LLVM

# 또는 Visual Studio Build Tools 설치
# https://visualstudio.microsoft.com/downloads/
```

## Linux 환경 설정

### 1. Rust 설치

```bash
# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 환경 변수 로드
source $HOME/.cargo/env
```

### 2. 필수 도구 설치

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    nasm \
    qemu-system-x86 \
    qemu-utils \
    ovmf \
    gdb

# Fedora
sudo dnf install -y \
    gcc \
    nasm \
    qemu-system-x86 \
    qemu-utils \
    edk2-ovmf \
    gdb
```

### 3. Rust 설정

```bash
# Nightly Rust 설치
rustup install nightly
rustup default nightly

# 필수 컴포넌트 추가
rustup component add rust-src llvm-tools-preview

# x86_64 타겟 추가
rustup target add x86_64-unknown-none

# bootimage 설치
cargo install bootimage --version "^0.11.0"
```

## 설정 검증

### 1. Rust 버전 확인

```bash
# Windows
rustc --version --verbose
rustup show

# 설치된 타겟 확인
rustup target list --installed
```

예상 출력:
```
rustc 1.xx.0-nightly (xxxxx xxxx-xx-xx)
Default host: x86_64-pc-windows-msvc (또는 x86_64-unknown-linux-gnu)
installed toolchains:
  nightly-x86_64-pc-windows-msvc (active)
installed targets:
  x86_64-unknown-none
  x86_64-pc-windows-msvc
```

### 2. bootimage 확인

```bash
cargo --list | grep bootimage
```

### 3. QEMU 확인

```bash
# Windows
qemu-system-x86_64 --version

# Linux
qemu-system-x86_64 --version
```

### 4. 프로젝트 빌드 테스트

```bash
# 프로젝트 루트에서
cargo build

# 빌드가 성공하면 다음 단계로 진행 가능
```

## 문제 해결

### 문제: bootimage 설치 실패

**해결 방법:**
1. LLVM이 설치되어 있는지 확인
2. Windows의 경우 Visual Studio Build Tools 설치
3. 재시도: `cargo install bootimage --version "^0.11.0"`

### 문제: 크로스 컴파일 오류

**해결 방법:**
1. `.cargo/config.toml` 파일이 올바른지 확인
2. x86_64-unknown-none 타겟이 설치되어 있는지 확인
3. LLVM/clang이 PATH에 있는지 확인

### 문제: QEMU 실행 오류

**해결 방법:**
1. QEMU가 올바르게 설치되었는지 확인
2. PATH 환경 변수에 QEMU 경로가 포함되어 있는지 확인
3. Windows의 경우 관리자 권한으로 실행 시도

### 문제: 시리얼 포트 출력이 보이지 않음

**해결 방법:**
1. QEMU 실행 시 `-serial stdio` 옵션이 있는지 확인
2. `scripts/run.ps1` 또는 `scripts/run.sh` 스크립트 사용

## 다음 단계

환경 설정이 완료되면:

1. **프로젝트 빌드**: `cargo build`
2. **QEMU에서 실행**: `.\scripts\run.ps1` (Windows) 또는 `./scripts/run.sh` (Linux)
3. **디버깅**: `.\scripts\debug.ps1` (Windows) 또는 `./scripts/debug.sh` (Linux)

## 추가 리소스

- [Rust 공식 문서](https://doc.rust-lang.org/)
- [Writing an OS in Rust](https://os.phil-opp.com/)
- [QEMU 문서](https://www.qemu.org/documentation/)
- [bootimage 크레이트](https://docs.rs/bootimage/)

---

**마지막 업데이트**: 2024년

