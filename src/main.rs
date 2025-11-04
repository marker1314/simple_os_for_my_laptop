//! Simple OS Kernel Entry Point
//!
//! 이 파일은 커널의 엔트리 포인트입니다.
//! 부트로더가 커널을 로드한 후 `_start` 함수가 호출됩니다.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(simple_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

/// 패닉 핸들러
///
/// 커널 패닉이 발생했을 때 호출됩니다.
/// 현재는 무한 루프에 빠지지만, 향후 로깅 및 복구 기능을 추가할 예정입니다.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // TODO: 시리얼 포트나 VGA를 통한 패닉 메시지 출력
    loop {
        // 패닉 발생 시 무한 루프
        x86_64::instructions::hlt();
    }
}

/// 커널 엔트리 포인트
///
/// 부트로더가 커널을 로드한 후 이 함수가 호출됩니다.
/// 초기화 순서:
/// 1. 인터럽트 디스크립터 테이블 (IDT) 설정
/// 2. 메모리 관리자 초기화
/// 3. 스케줄러 초기화
/// 4. 전력 관리 초기화
/// 5. 드라이버 초기화
/// 6. Shell/GUI 시작
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 커널 초기화
    kernel_init();
    
    // 메인 루프
    loop {
        // 유휴 상태에서 CPU를 대기 상태로 전환 (전력 절약)
        x86_64::instructions::hlt();
    }
}

/// 커널 초기화 함수
///
/// 모든 커널 모듈을 순서대로 초기화합니다.
fn kernel_init() {
    // TODO: 초기화 순서에 따라 각 모듈 초기화
    // 1. IDT 설정
    // 2. 메모리 관리자 초기화
    // 3. 힙 할당자 초기화
    // 4. 인터럽트 활성화
    // 5. 드라이버 초기화
    // 6. 스케줄러 시작
    // 7. 전력 관리 초기화
    // 8. Shell/GUI 시작
}

