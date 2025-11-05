//! 부트로더 인터페이스 및 커널 초기화 모듈
//!
//! 이 모듈은 부트로더와의 인터페이스 및 커널 초기화를 담당합니다.

pub mod info;
pub mod timeline;

pub use info::{init as init_boot_info, get as get_boot_info, memory_map_len, acpi_rsdp_addr};
pub use timeline::{mark_boot_start, mark_stage, print_timeline, export_timeline_csv, get_total_boot_time_ms, BootStage};
