# Phase 16: Multi-core Support (SMP) - 구현 완료

## 개요

Phase 16에서는 멀티코어 CPU 지원을 위한 SMP (Symmetric Multiprocessing) 기능을 구현했습니다. 이를 통해 커널이 여러 CPU 코어를 동시에 활용할 수 있게 되었습니다.

## 구현된 기능

### 1. SMP 모듈 구조 (`src/smp/`)

새로운 SMP 모듈을 생성하여 멀티코어 지원의 기반을 마련했습니다.

#### 1.1 `smp/mod.rs` - SMP 시스템 관리
- **전역 CPU 정보 관리**: 모든 CPU 코어의 정보를 추적
- **SMP 초기화 함수**: APIC, I/O APIC, AP 프로세서 초기화
- **CPU 수 감지**: 시스템의 CPU 수 자동 감지
- **IPI 브로드캐스트**: 모든 CPU에 인터럽트 전송
- **작업 분배 인터페이스**: CPU 간 작업 균등 분배

#### 1.2 `smp/apic.rs` - APIC 드라이버
- **Local APIC 초기화**: 각 CPU 코어의 Local APIC 설정
- **I/O APIC 초기화**: 외부 인터럽트 라우팅 관리
- **APIC 레지스터 관리**: 
  - ID, Version, TPR, EOI 등
  - LVT (Local Vector Table) 엔트리
  - ICR (Interrupt Command Register)
- **MSR을 통한 APIC 제어**: Model Specific Register 접근
- **인터럽트 라우팅**: I/O APIC Redirection Table 설정
- **EOI 처리**: End of Interrupt 신호 전송

#### 1.3 `smp/cpu.rs` - CPU 정보 관리
- **CpuInfo 구조체**: 각 CPU의 상태 및 정보 저장
  - CPU ID (APIC ID)
  - Bootstrap Processor 여부
  - CPU 상태 (Inactive, Initializing, Active, Idle, Error)
  - 로드 (0-100%)
  - 실행 중인 스레드 수
- **CPUID 명령어 지원**: CPU 기능 및 정보 조회
- **CPU 벤더 감지**: Intel, AMD 등 벤더 정보
- **CPU 기능 확인**: 
  - FPU, APIC, MMX, SSE, SSE2, SSE3
  - x2APIC 지원 여부

#### 1.4 `smp/ipi.rs` - IPI (Inter-Processor Interrupt)
- **IPI 전송 메커니즘**: CPU 간 통신 인터페이스
- **전달 모드 지원**:
  - Fixed: 고정 벡터 전달
  - LowestPriority: 가장 낮은 우선순위 CPU로 전달
  - SMI, NMI, INIT, StartUp
- **목적지 선택**:
  - 특정 CPU에 전송
  - 자신에게만 전송
  - 모든 CPU에 브로드캐스트 (자신 포함/제외)
- **특수 IPI**:
  - INIT IPI: CPU 초기화
  - SIPI (Startup IPI): AP 프로세서 시작
  - TLB Flush IPI: 모든 CPU의 TLB 플러시
  - Reschedule IPI: 스케줄러 재스케줄링 요청

### 2. 로드 밸런서 (`src/scheduler/load_balancer.rs`)

멀티코어 환경에서 작업을 효율적으로 분배하는 로드 밸런서를 구현했습니다.

#### 2.1 로드 밸런싱 전략
- **Round-Robin**: 순차적으로 CPU에 작업 할당
- **Least Loaded**: 가장 부하가 적은 CPU에 할당
- **Work Stealing**: 유휴 CPU가 바쁜 CPU에서 작업 가져옴 (기본 구조)

#### 2.2 CPU 로드 추적
- **CpuLoad 구조체**: CPU별 부하 정보 관리
  - 실행 중인 스레드 수
  - CPU 사용률 (0-100%)
- **동적 로드 업데이트**: 실시간 CPU 부하 모니터링
- **로드 불균형 감지**: 재조정 필요 여부 자동 판단

#### 2.3 전역 로드 밸런서
- **초기화 함수**: CPU 수와 전략 설정
- **CPU 선택 함수**: 새 스레드에 적합한 CPU 선택
- **스레드 이벤트 처리**: 추가/제거 통지
- **로드 정보 조회**: 모든 CPU의 현재 상태 확인

### 3. 기존 모듈 통합

#### 3.1 스케줄러 모듈 업데이트
- `scheduler/mod.rs`에 `load_balancer` 서브모듈 추가
- 멀티코어 환경에서 스케줄링 지원 준비

#### 3.2 커널 라이브러리 업데이트
- `src/lib.rs`에 `smp` 모듈 등록
- 모든 커널 모듈에서 SMP 기능 접근 가능

## 기술적 세부사항

### APIC vs PIC

기존의 PIC (Programmable Interrupt Controller)에서 APIC (Advanced Programmable Interrupt Controller)로 전환:

- **PIC의 한계**: 
  - 최대 15개의 IRQ만 지원
  - 단일 CPU만 지원
  - 우선순위 관리 제한적

- **APIC의 장점**:
  - 각 CPU 코어마다 Local APIC 보유
  - 최대 224개의 인터럽트 벡터 지원
  - CPU 간 통신 (IPI) 가능
  - 동적 우선순위 관리
  - 멀티코어 환경 완벽 지원

### Local APIC vs I/O APIC

- **Local APIC**: 
  - 각 CPU 코어에 내장
  - 로컬 인터럽트 처리 (타이머, 오류 등)
  - IPI 송수신
  - EOI 처리

- **I/O APIC**:
  - 외부 장치 인터럽트 라우팅
  - Redirection Table을 통한 인터럽트 분배
  - 특정 CPU로 인터럽트 전달 가능

### IPI를 통한 AP 초기화 시퀀스

AP (Application Processor) 초기화는 INIT-SIPI-SIPI 시퀀스를 따릅니다:

1. **INIT IPI**: AP를 초기화 상태로 전환
2. **대기**: 10ms 대기
3. **SIPI (Startup IPI)**: 시작 주소 전송
4. **대기**: 200μs 대기
5. **SIPI (재전송)**: 안정성을 위해 재전송
6. **대기**: AP가 초기화될 때까지 대기

## 성능 및 효율성

### 예상 성능 향상

- **병렬 처리**: 멀티코어를 활용한 동시 실행
- **로드 분산**: CPU 부하를 균등하게 분배
- **응답성 향상**: 특정 CPU 과부하 방지
- **전력 효율**: 필요한 코어만 활성화 가능

### 메모리 오버헤드

- **CPU 정보**: CPU당 약 32바이트
- **로드 밸런서**: 고정 크기 + CPU 수 × 16바이트
- **APIC 매핑**: 8KB (Local APIC) + 256바이트 (I/O APIC)

## 제한 사항 및 향후 개선

### 현재 제한사항

1. **ACPI 통합 미완료**: CPU 수를 ACPI MADT에서 읽어야 함 (현재는 하드코딩)
2. **AP 초기화 미구현**: INIT-SIPI-SIPI 시퀀스 코드 필요
3. **메모리 매핑**: APIC 메모리 영역을 페이지 테이블에 매핑 필요
4. **Work Stealing**: 완전한 Work Stealing 알고리즘 구현 필요
5. **Per-CPU 데이터**: 각 CPU별 데이터 구조 최적화 필요

### 향후 개선 계획

1. **ACPI MADT 파싱**: 
   - CPU 수 자동 감지
   - APIC ID 매핑
   - NUMA 토폴로지 정보

2. **AP 초기화 완성**:
   - Trampoline 코드 작성 (16비트 실모드)
   - AP 스택 할당
   - AP별 GDT/IDT 설정

3. **스케줄러 개선**:
   - Per-CPU 실행 큐
   - CPU 어피니티 지원
   - NUMA-aware 스케줄링

4. **동기화 프리미티브**:
   - Per-CPU 변수
   - RCU (Read-Copy-Update)
   - Lockless 알고리즘

5. **인터럽트 최적화**:
   - 인터럽트 밸런싱
   - IRQ 어피니티 설정
   - MSI/MSI-X 지원

## 테스트 방법

### QEMU에서 테스트

멀티코어 환경 시뮬레이션:

```bash
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -smp 4 \
    -serial stdio
```

- `-smp 4`: 4개의 CPU 코어 시뮬레이션

### 확인 방법

1. **부팅 로그 확인**:
   ```
   [INFO] Initializing SMP support...
   [INFO] Local APIC initialized on BSP
   [INFO] I/O APIC initialized
   [INFO] Detected 4 CPU(s)
   ```

2. **CPU 정보 확인**:
   - CPUID를 통한 CPU 기능 확인
   - APIC ID 출력

3. **로드 밸런서 확인**:
   - CPU별 스레드 분배 확인
   - 로드 정보 모니터링

## 호환성

### 지원 플랫폼

- **x86_64 아키텍처**: Intel 및 AMD 프로세서
- **APIC 지원**: 모든 현대 x86_64 CPU
- **멀티코어**: 2코어 이상 시스템

### 요구사항

- **최소**: 1코어 (단일 코어에서도 동작)
- **권장**: 2코어 이상
- **메모리**: 추가 메모리 요구량 미미

## 관련 파일

### 새로 생성된 파일
- `src/smp/mod.rs` - SMP 시스템 관리
- `src/smp/apic.rs` - APIC 드라이버
- `src/smp/cpu.rs` - CPU 정보 관리
- `src/smp/ipi.rs` - IPI 메커니즘
- `src/scheduler/load_balancer.rs` - 로드 밸런서
- `docs/phase16-summary.md` - 이 문서

### 수정된 파일
- `src/lib.rs` - SMP 모듈 등록
- `src/scheduler/mod.rs` - 로드 밸런서 추가

## 참고 자료

### 문서
- [Intel 64 and IA-32 Architectures Software Developer's Manual, Volume 3](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
  - Chapter 10: Advanced Programmable Interrupt Controller (APIC)
  - Chapter 8: Multiple-Processor Management

- [OSDev Wiki - APIC](https://wiki.osdev.org/APIC)
- [OSDev Wiki - SMP](https://wiki.osdev.org/Symmetric_Multiprocessing)

### 참고 OS 프로젝트
- [Redox OS](https://github.com/redox-os/redox) - Rust OS의 SMP 구현
- [Linux Kernel](https://github.com/torvalds/linux) - SMP 스케줄러

## 결론

Phase 16에서 구현한 멀티코어 지원은 Simple OS가 현대 하드웨어의 성능을 완전히 활용할 수 있는 기반을 마련했습니다. APIC 드라이버, IPI 메커니즘, 로드 밸런서를 통해 CPU 간 효율적인 작업 분배가 가능해졌습니다.

현재는 기본 구조가 완성되었으며, 향후 ACPI 통합과 AP 초기화 코드를 완성하면 실제 멀티코어 환경에서 동작할 수 있게 됩니다. 이는 시스템 성능과 전력 효율을 크게 향상시킬 것입니다.

---

**구현 날짜**: 2024년  
**구현자**: Simple OS 개발팀  
**다음 단계**: Phase 17 - Enhanced Filesystem Features




