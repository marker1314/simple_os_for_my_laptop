//! 안전성 검증 모듈
//!
//! 이 모듈은 unsafe 블록 검증, 메모리 안전성 검사 등을 제공합니다.

pub mod unsafe_validator;
#[macro_use]
pub mod macros;

pub use unsafe_validator::{
    UnsafeBlockType, UnsafeContext, PointerValidator, HardwareValidator,
    UnsafeStats, record_unsafe_block, print_unsafe_stats,
};

