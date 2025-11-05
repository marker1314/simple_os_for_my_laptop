# Unsafe 블록 검증 가이드

## 개요

Simple OS는 284개의 unsafe 블록을 포함하고 있습니다. 이는 하드웨어 접근, 메모리 관리 등에 필수적이지만, 적절한 검증이 필요합니다.

## 검증 모듈 사용법

### 1. 포인터 검증

```rust
use simple_os::safety::PointerValidator;

// 기존 방식
unsafe {
    let value = *(ptr as *const u32);
}

// 검증된 방식
let validator = PointerValidator::new(ptr, 4);
unsafe {
    validator.validate()?;
    let value = *validator.as_ptr::<u32>();
}
```

### 2. 하드웨어 레지스터 접근

```rust
use simple_os::safety::HardwareValidator;

// 기존 방식
unsafe {
    let virt_addr = phys_offset + phys_addr;
    let value = *(virt_addr as *const u32);
}

// 검증된 방식
let validator = HardwareValidator::new(phys_addr, 4);
unsafe {
    validator.validate()?;
    let virt_addr = validator.to_virt_addr();
    let value = *(virt_addr as *const u32);
}
```

### 3. 매크로 사용

```rust
use simple_os::{unsafe_checked, UnsafeBlockType};

// 검증된 unsafe 블록
unsafe_checked! {
    type: UnsafeBlockType::HardwareAccess,
    desc: "MMIO 레지스터 읽기",
    validate: {
        HardwareValidator::new(phys_addr, 4).validate()
    },
    block: {
        let virt_addr = phys_offset + phys_addr;
        *(virt_addr as *const u32)
    }
}
```

## 검증 통계

커널 종료 시 unsafe 블록 통계를 출력할 수 있습니다:

```rust
use simple_os::safety::print_unsafe_stats;

// 커널 종료 전
print_unsafe_stats();
```

## Best Practices

1. **포인터 역참조 전 항상 검증**
   - Null 포인터 검사
   - 범위 검사
   - 커널 공간 접근 방지

2. **하드웨어 접근 시 검증**
   - 물리 주소 유효성
   - MMIO 범위 확인
   - 접근 크기 검증 (1, 2, 4, 8 바이트만 허용)

3. **문서화**
   - unsafe 블록의 목적 명시
   - 사전 조건 (preconditions) 문서화
   - 사후 조건 (postconditions) 문서화

4. **통계 추적**
   - 검증된 블록과 검증되지 않은 블록 구분
   - 정기적으로 통계 확인

## 미래 개선 사항

1. 컴파일 타임 검증 강화
2. 런타임 검증 자동화
3. 정적 분석 도구 통합
4. 검증 패턴 라이브러리 확장

