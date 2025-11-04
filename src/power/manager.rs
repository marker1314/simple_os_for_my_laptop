//! 전력 관리자
//!
//! 전력 관리의 중앙 제어를 담당합니다.

use crate::power::acpi::AcpiParser;
use crate::power::scaling::CpuScaling;
use crate::power::policy::{PowerPolicy, PowerMode};
use crate::power::PowerError;

/// 전력 관리자
///
/// CPU 전력 관리를 위한 중앙 관리자입니다.
pub struct PowerManager {
    /// ACPI 파서
    acpi_parser: Option<AcpiParser>,
    /// CPU 스케일링 관리자
    cpu_scaling: CpuScaling,
    /// 현재 전력 정책
    policy: PowerPolicy,
    /// 초기화 여부
    initialized: bool,
}

impl PowerManager {
    /// 새 전력 관리자 생성
    pub fn new() -> Result<Self, PowerError> {
        // Choose default power mode based on build profile features
        let default_mode = match crate::config::profile::current_profile() {
            crate::config::profile::Profile::PowerSaver => PowerMode::PowerSaving,
            crate::config::profile::Profile::Performance => PowerMode::Performance,
            _ => PowerMode::Balanced,
        };
        Ok(Self {
            acpi_parser: None,
            cpu_scaling: CpuScaling::new(),
            policy: PowerPolicy::new(default_mode),
            initialized: false,
        })
    }
    
    /// 전력 관리자 초기화
    ///
    /// ACPI 테이블을 파싱하고 전력 관리를 설정합니다.
    ///
    /// # Safety
    /// 메모리 관리가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), PowerError> {
        // ACPI 파서 초기화 시도
        match AcpiParser::new() {
            Ok(parser) => {
                let mut parser = parser;
                // Initialize ACPI to allow minimal table queries
                match parser.init() {
                    Ok(()) => {
                        crate::log_info!("ACPI parser initialized");
                        // Populate idle C-state hints
                        let hints = parser.discover_c_states();
                        let mut table: [Option<crate::power::idle::CState>; 8] = [None, None, None, None, None, None, None, None];
                        for (i, opt) in hints.iter().enumerate() {
                            if let Some(h) = opt {
                                table[i] = Some(crate::power::idle::CState { level: h.level, latency_us: h.latency_us, power_mw: h.power_mw, mwait_hint: h.mwait_hint });
                            }
                        }
                        crate::power::idle::set_default_cstates(table);
                    }
                    Err(e) => {
                        crate::log_warn!("ACPI init failed: {:?}", e);
                    }
                }
                self.acpi_parser = Some(parser);
            }
            Err(e) => {
                crate::log_warn!("Failed to initialize ACPI parser: {:?}", e);
                // ACPI가 없어도 기본 전력 관리는 가능
            }
        }
        
        // CPU 스케일링 초기화
        self.cpu_scaling.init()?;
        
        self.initialized = true;
        Ok(())
    }
    
    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// 전력 정책 설정
    pub fn set_policy(&mut self, mode: PowerMode) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        self.policy.set_mode(mode);
        
        // 정책에 따라 CPU 스케일링 조정
        match mode {
            PowerMode::Performance => {
                self.cpu_scaling.set_max_performance()?;
            }
            PowerMode::Balanced => {
                self.cpu_scaling.set_balanced()?;
            }
            PowerMode::PowerSaving => {
                self.cpu_scaling.set_power_saving()?;
            }
        }
        
        Ok(())
    }
    
    /// 현재 전력 정책 가져오기
    pub fn get_policy(&self) -> PowerMode {
        self.policy.get_mode()
    }
    
    /// CPU 유휴 상태 진입
    ///
    /// 유휴 상태에서 CPU를 최대한 절전 모드로 전환합니다.
    ///
    /// # Safety
    /// 인터럽트가 활성화된 상태에서 호출되어야 합니다.
    pub unsafe fn enter_idle(&self) {
        // 가장 깊은 가능한 C-State로 진입 (가능하면 MWAIT, 아니면 HLT)
        // 인터럽트는 활성 상태여야 함
        // 간단한 내장 관리자를 사용 (향후 ACPI 기반 채움)
        let idle = crate::power::idle::IdleStateManager::new();
        idle.enter_deepest();
    }
    
    /// CPU 사용률에 따른 동적 스케일링
    ///
    /// CPU 사용률이 낮으면 클럭을 낮추고, 높으면 클럭을 높입니다.
    pub fn update_scaling(&mut self, cpu_usage_percent: u8) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        // CPU 사용률에 따라 스케일링 조정
        if cpu_usage_percent < 20 {
            // 사용률이 낮으면 절전 모드로
            self.cpu_scaling.set_power_saving()?;
        } else if cpu_usage_percent > 80 {
            // 사용률이 높으면 성능 모드로
            self.cpu_scaling.set_max_performance()?;
        } else {
            // 중간이면 균형 모드로
            self.cpu_scaling.set_balanced()?;
        }
        
        Ok(())
    }
}

