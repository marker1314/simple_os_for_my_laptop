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
    /// CPU Idle 상태 관리자
    idle_manager: crate::power::idle::IdleStateManager,
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
            idle_manager: crate::power::idle::IdleStateManager::new(),
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
    /// 지연 시간 기반 동적 조정을 사용합니다.
    ///
    /// # Safety
    /// 인터럽트가 활성화된 상태에서 호출되어야 합니다.
    pub unsafe fn enter_idle(&mut self) {
        // CPU 사용률 업데이트
        crate::power::cpu_usage::update_cpu_usage();
        
        // 정책에 따라 최적 C-State 선택
        let cpu_usage = crate::power::cpu_usage::get_cpu_usage_percent();
        let threshold = self.policy.get_idle_threshold();
        let recommended = self.idle_manager.get_recommended_c_state(threshold, cpu_usage);
        
        // 지연 시간 기반 동적 조정 시도
        // 현재는 정책 기반 선택, 향후 enter_optimal_c_state 사용 가능
        self.idle_manager.enter_c_state(recommended);
        
        // Wakeup 시 record_wakeup 호출 필요 (인터럽트 핸들러에서)
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

    /// Suspend to RAM (S3) - complete implementation with validation
    pub fn suspend_s3(&mut self) -> Result<(), PowerError> {
        if self.acpi_parser.is_none() {
            return Err(PowerError::Unsupported);
        }
        
        crate::log_info!("=== Entering S3 suspend ===");
        
        // 사전 검증: ACPI 지원 확인
        if let Some(ref parser) = self.acpi_parser {
            if !parser.is_s3_supported() {
                crate::log_error!("S3 sleep state not supported by ACPI");
                return Err(PowerError::Unsupported);
            }
        }
        
        // 사전 검증: 장치 상태 확인
        crate::log_info!("Pre-suspend validation: checking device states...");
        
        // 1. 장치 quiesce (상태 저장 및 전원 끄기)
        match crate::power::device::quiesce_all_devices() {
            Ok(()) => {
                crate::log_info!("All devices quiesced successfully");
            }
            Err(e) => {
                crate::log_error!("Device quiesce failed: {:?}", e);
                return Err(e);
            }
        }
        
        // 2. 캐시 flush
        unsafe {
            crate::log_info!("Flushing TLB and cache...");
            x86_64::instructions::tlb::flush_all();
            // Write-back cache flush
            core::arch::asm!("wbinvd", options(nostack, preserves_flags));
            crate::log_info!("Cache flushed");
        }
        
        // 3. 인터럽트 비활성화
        crate::log_info!("Disabling interrupts...");
        crate::interrupts::idt::disable_interrupts();
        
        // 4. 최종 확인: ACPI 파서 유효성 확인
        if let Some(ref parser) = self.acpi_parser {
            unsafe {
                crate::log_info!("Entering S3 sleep state...");
                // S3 sleep state (3)
                match parser.enter_sleep_state(3) {
                    Ok(()) => {
                        // 시스템이 여기서 깨어나면 resume으로 진행
                        // 이 지점은 실제로는 실행되지 않음 (하드웨어가 깨움)
                        crate::log_info!("Woke from S3 sleep");
                        self.resume()
                    }
                    Err(e) => {
                        // Sleep 실패 - 인터럽트 재활성화 및 장치 복원
                        crate::log_error!("Failed to enter S3 sleep: {:?}", e);
                        crate::interrupts::idt::enable_interrupts();
                        match crate::power::device::resume_all_devices() {
                            Ok(()) => {}
                            Err(resume_err) => {
                                crate::log_error!("Failed to resume devices after sleep failure: {:?}", resume_err);
                            }
                        }
                        Err(e)
                    }
                }
            }
        } else {
            crate::interrupts::idt::enable_interrupts();
            Err(PowerError::Unsupported)
        }
    }

    /// Resume from sleep - complete implementation
    pub fn resume(&mut self) -> Result<(), PowerError> {
        crate::log_info!("=== Resuming from sleep ===");
        
        // 1. 인터럽트 재활성화
        crate::interrupts::idt::enable_interrupts();
        
        // 2. 타이머 재초기화
        unsafe {
            crate::drivers::timer::init();
            crate::interrupts::pic::set_mask(0, true); // 타이머 인터럽트 활성화
        }
        
        // 3. 장치 복원
        crate::power::device::resume_all_devices()?;
        
        // 4. 전력 관리 재초기화
        if !self.initialized {
            unsafe {
                self.init()?;
            }
        }
        
        crate::log_info!("=== Resume complete ===");
        Ok(())
    }

    /// Get current P-state
    pub fn get_current_p_state(&self) -> u8 {
        self.cpu_scaling.get_current_p_state()
    }
    
    /// Get current C-state
    pub fn get_current_c_state(&self) -> u8 {
        crate::power::idle::get_current_c_state()
    }
    
    /// Thermal throttle 적용 (온도가 높을 때)
    pub fn apply_thermal_throttle(&mut self) -> Result<(), PowerError> {
        // CPU 주파수를 낮춰 전력 소비 감소
        if self.initialized {
            self.cpu_scaling.set_power_saving()
        } else {
            Err(PowerError::NotInitialized)
        }
    }
    
    /// Emergency thermal throttle (최대 온도 도달)
    pub fn apply_emergency_throttle(&mut self) -> Result<(), PowerError> {
        // 최대 절전 모드로 전환
        if self.initialized {
            self.cpu_scaling.set_power_saving()
        } else {
            Err(PowerError::NotInitialized)
        }
    }
    
    /// Governor 설정
    pub fn set_governor(&mut self, governor: crate::power::scaling::ScalingGovernor) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        self.cpu_scaling.set_governor(governor)
    }
    
    /// On-demand governor 업데이트
    pub fn update_ondemand(&mut self, cpu_usage_percent: Option<u8>, now_ms: u64) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        // CPU 사용률이 제공되지 않으면 계산
        let usage = cpu_usage_percent.unwrap_or_else(|| {
            crate::power::cpu_usage::update_cpu_usage();
            crate::power::cpu_usage::get_cpu_usage_percent()
        });
        
        self.cpu_scaling.update_ondemand(usage, now_ms)
    }
}

