//! xHCI Transfer Request Block (TRB) 구조
//!
//! xHCI 명령 및 전송을 위한 TRB 구조를 정의합니다.

/// TRB 타입
#[repr(u8)]
pub enum TrbType {
    /// Reserved
    Reserved = 0,
    /// Normal Transfer
    Normal = 1,
    /// Setup Stage
    SetupStage = 2,
    /// Data Stage
    DataStage = 3,
    /// Status Stage
    StatusStage = 4,
    /// Isoch Transfer
    Isoch = 5,
    /// Link TRB
    Link = 6,
    /// Event Data
    EventData = 7,
    /// No-Op
    NoOp = 8,
    /// Enable Slot Command
    EnableSlot = 9,
    /// Disable Slot Command
    DisableSlot = 10,
    /// Address Device Command
    AddressDevice = 11,
    /// Configure Endpoint Command
    ConfigureEndpoint = 12,
    /// Evaluate Context Command
    EvaluateContext = 13,
    /// Reset Endpoint Command
    ResetEndpoint = 14,
    /// Stop Endpoint Command
    StopEndpoint = 15,
    /// Set TR Dequeue Pointer
    SetTrDequeuePointer = 16,
    /// Reset Device Command
    ResetDevice = 17,
    /// Force Event Command
    ForceEvent = 18,
    /// Negotiate Bandwidth Command
    NegotiateBandwidth = 19,
    /// Set Latency Tolerance Value Command
    SetLatencyToleranceValue = 20,
    /// Get Port Bandwidth Command
    GetPortBandwidth = 21,
    /// Force Header Command
    ForceHeader = 22,
    /// No-Op Command
    NoOpCommand = 23,
    /// Get Extended Property Command
    GetExtendedProperty = 24,
    /// Set Extended Property Command
    SetExtendedProperty = 25,
}

/// TRB Control 필드
#[repr(u32)]
pub struct TrbControl {
    /// Cycle bit (비트 0)
    cycle: bool,
    /// Evaluate Next TRB (비트 1)
    evaluate_next: bool,
    /// Interrupt On Completion (비트 2)
    interrupt_on_completion: bool,
    /// Immediate Data (비트 3)
    immediate_data: bool,
    /// TRB Type (비트 4-15)
    trb_type: u8,
    /// Reserved (비트 16-23)
    reserved1: u8,
    /// Reserved (비트 24-31)
    reserved2: u8,
}

impl TrbControl {
    /// 새 TRB Control 생성
    pub fn new(trb_type: TrbType, interrupt_on_completion: bool) -> Self {
        Self {
            cycle: true,
            evaluate_next: false,
            interrupt_on_completion,
            immediate_data: false,
            trb_type: trb_type as u8,
            reserved1: 0,
            reserved2: 0,
        }
    }
    
    /// u32로 변환
    pub fn to_u32(&self) -> u32 {
        let mut value = 0u32;
        if self.cycle {
            value |= 1 << 0;
        }
        if self.evaluate_next {
            value |= 1 << 1;
        }
        if self.interrupt_on_completion {
            value |= 1 << 2;
        }
        if self.immediate_data {
            value |= 1 << 3;
        }
        value |= (self.trb_type as u32) << 4;
        value |= (self.reserved1 as u32) << 16;
        value |= (self.reserved2 as u32) << 24;
        value
    }
}

/// TRB (Transfer Request Block) - 16바이트
#[repr(C, packed)]
pub struct Trb {
    /// Parameter 0 (비트 0-31)
    pub parameter0: u32,
    /// Parameter 1 (비트 32-63)
    pub parameter1: u32,
    /// Parameter 2 (비트 64-95)
    pub parameter2: u32,
    /// Control (비트 96-127)
    pub control: u32,
}

impl Trb {
    /// 새 TRB 생성
    pub fn new() -> Self {
        Self {
            parameter0: 0,
            parameter1: 0,
            parameter2: 0,
            control: 0,
        }
    }
    
    /// Normal Transfer TRB 생성
    pub fn new_normal_transfer(
        data_buffer: u64,
        transfer_length: u32,
        interrupt_on_completion: bool,
    ) -> Self {
        let mut trb = Self::new();
        trb.parameter0 = (data_buffer & 0xFFFF_FFFF) as u32;
        trb.parameter1 = ((data_buffer >> 32) & 0xFFFF_FFFF) as u32;
        trb.parameter2 = transfer_length;
        
        let control = TrbControl::new(TrbType::Normal, interrupt_on_completion);
        trb.control = control.to_u32();
        
        trb
    }
    
    /// Setup Stage TRB 생성
    pub fn new_setup_stage(setup_data: &[u8; 8], interrupt_on_completion: bool) -> Self {
        let mut trb = Self::new();
        
        // Setup Data (8바이트)를 Parameter 0-1에 저장
        trb.parameter0 = u32::from_le_bytes([setup_data[0], setup_data[1], setup_data[2], setup_data[3]]);
        trb.parameter1 = u32::from_le_bytes([setup_data[4], setup_data[5], setup_data[6], setup_data[7]]);
        
        // Transfer Length = 8 (Setup Stage)
        trb.parameter2 = 8;
        
        let control = TrbControl::new(TrbType::SetupStage, interrupt_on_completion);
        trb.control = control.to_u32();
        
        trb
    }
    
    /// Data Stage TRB 생성
    pub fn new_data_stage(
        data_buffer: u64,
        transfer_length: u32,
        direction: bool, // true = IN, false = OUT
        interrupt_on_completion: bool,
    ) -> Self {
        let mut trb = Self::new_normal_transfer(data_buffer, transfer_length, interrupt_on_completion);
        
        // Control 필드 수정 (Direction bit)
        let mut control = TrbControl::new(TrbType::DataStage, interrupt_on_completion);
        // Direction은 TRB Control에 없고, Setup Stage의 요청에 따라 결정됨
        // 여기서는 간단히 구현
        trb.control = control.to_u32();
        
        trb
    }
    
    /// Status Stage TRB 생성
    pub fn new_status_stage(direction: bool, interrupt_on_completion: bool) -> Self {
        let mut trb = Self::new();
        trb.parameter0 = 0;
        trb.parameter1 = 0;
        trb.parameter2 = 0;
        
        let control = TrbControl::new(TrbType::StatusStage, interrupt_on_completion);
        trb.control = control.to_u32();
        
        trb
    }
    
    /// Link TRB 생성 (Command Ring 연결용)
    pub fn new_link(ring_segment_address: u64, toggle_cycle: bool) -> Self {
        let mut trb = Self::new();
        trb.parameter0 = (ring_segment_address & 0xFFFF_FFFF) as u32;
        trb.parameter1 = ((ring_segment_address >> 32) & 0xFFFF_FFFF) as u32;
        trb.parameter2 = 0;
        
        let mut control = TrbControl::new(TrbType::Link, false);
        control.cycle = toggle_cycle;
        trb.control = control.to_u32();
        
        trb
    }
    
    /// Address Device Command TRB 생성
    pub fn new_address_device(slot_id: u8, input_context_address: u64) -> Self {
        let mut trb = Self::new();
        trb.parameter0 = (input_context_address & 0xFFFF_FFFF) as u32;
        trb.parameter1 = ((input_context_address >> 32) & 0xFFFF_FFFF) as u32;
        trb.parameter2 = (slot_id as u32) << 24;
        
        let control = TrbControl::new(TrbType::AddressDevice, true);
        trb.control = control.to_u32();
        
        trb
    }
}

impl Default for Trb {
    fn default() -> Self {
        Self::new()
    }
}

