//! HDA 코덱 관리
//!
//! HDA 코덱 초기화 및 Verb 명령 전송을 담당합니다.

use crate::drivers::audio::AudioError;

/// 코덱 번호
pub type CodecId = u8;

/// HDA Verb 명령 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdaVerbCommand {
    /// Get Parameter (읽기)
    GetParameter(u16, u16), // Node ID, Parameter ID
    /// Set Control (쓰기)
    SetControl(u16, u16, u16), // Node ID, Control ID, Value
    /// Get Connection Select
    GetConnectionSelect(u16),
    /// Set Connection Select
    SetConnectionSelect(u16, u8),
    /// Get Amplifier Gain
    GetAmplifierGain(u16, u8, u8), // Node ID, Direction, Index
    /// Set Amplifier Gain
    SetAmplifierGain(u16, u8, u8, u8), // Node ID, Direction, Index, Gain
    /// Get Converter Stream/Channel
    GetConverterStreamChannel(u16),
    /// Set Converter Stream/Channel
    SetConverterStreamChannel(u16, u8, u8), // Node ID, Stream, Channel
    /// Get Power State
    GetPowerState(u16, u8), // Node ID, Power State Index
    /// Set Power State
    SetPowerState(u16, u8, u8), // Node ID, Power State Index, Power State
    /// Get Connection List Entry
    GetConnectionListEntry(u16, u8), // Node ID, Index
    /// Get Processing Coefficient
    GetProcessingCoefficient(u16, u16, u8), // Node ID, Index, Coefficient Index
    /// Set Processing Coefficient
    SetProcessingCoefficient(u16, u16, u8, u16), // Node ID, Index, Coefficient Index, Value
}

impl HdaVerbCommand {
    /// Verb를 32비트 값으로 변환
    pub fn to_u32(&self, codec_id: CodecId) -> u32 {
        match *self {
            HdaVerbCommand::GetParameter(node_id, param_id) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF00 as u32) << 8 // GET_PARAMETER
                | (param_id as u32)
            }
            HdaVerbCommand::SetControl(node_id, ctrl_id, value) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x700 as u32) << 8 // SET_CONTROL
                | ((ctrl_id as u32) << 8)
                | (value as u32)
            }
            HdaVerbCommand::GetConnectionSelect(node_id) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF01 as u32) << 8 // GET_CONNECTION_SELECT
            }
            HdaVerbCommand::SetConnectionSelect(node_id, selector) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x701 as u32) << 8 // SET_CONNECTION_SELECT
                | (selector as u32)
            }
            HdaVerbCommand::GetAmplifierGain(node_id, direction, index) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF02 as u32) << 8 // GET_AMPLIFIER_GAIN
                | ((direction as u32) << 7)
                | ((index as u32) << 0)
            }
            HdaVerbCommand::SetAmplifierGain(node_id, direction, index, gain) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x702 as u32) << 8 // SET_AMPLIFIER_GAIN
                | ((direction as u32) << 7)
                | ((index as u32) << 0)
                | ((gain as u32) << 8)
            }
            HdaVerbCommand::GetConverterStreamChannel(node_id) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF04 as u32) << 8 // GET_CONVERTER_STREAM_CHANNEL
            }
            HdaVerbCommand::SetConverterStreamChannel(node_id, stream, channel) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x704 as u32) << 8 // SET_CONVERTER_STREAM_CHANNEL
                | ((stream as u32) << 4)
                | ((channel as u32) << 0)
            }
            HdaVerbCommand::GetPowerState(node_id, power_state) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF05 as u32) << 8 // GET_POWER_STATE
                | ((power_state as u32) << 0)
            }
            HdaVerbCommand::SetPowerState(node_id, power_state, state) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x705 as u32) << 8 // SET_POWER_STATE
                | ((power_state as u32) << 0)
                | ((state as u32) << 4)
            }
            HdaVerbCommand::GetConnectionListEntry(node_id, index) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF02 as u32) << 8 // GET_CONNECTION_LIST_ENTRY
                | ((index as u32) << 0)
            }
            HdaVerbCommand::GetProcessingCoefficient(node_id, index, coeff_index) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0xF20 as u32) << 8 // GET_PROCESSING_COEFFICIENT
                | ((index as u32) << 8)
                | ((coeff_index as u32) << 0)
            }
            HdaVerbCommand::SetProcessingCoefficient(node_id, index, coeff_index, value) => {
                (codec_id as u32) << 28
                | (node_id as u32) << 20
                | (0x720 as u32) << 8 // SET_PROCESSING_COEFFICIENT
                | ((index as u32) << 8)
                | ((coeff_index as u32) << 0)
                | ((value as u32) << 16)
            }
        }
    }
}

/// 코덱 파라미터 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecParameter {
    /// Vendor ID
    VendorId = 0x00,
    /// Revision ID
    RevisionId = 0x02,
    /// Subsystem ID
    SubsystemId = 0x04,
    /// Node Count
    NodeCount = 0x09,
    /// Function Group Type
    FunctionGroupType = 0x05,
    /// Audio Function Group Capabilities
    AudioFunctionGroupCap = 0x08,
    /// Audio Widget Capabilities
    AudioWidgetCap = 0x09,
    /// Supported PCM Rates
    SupportedPcmRates = 0x0A,
    /// Supported PCM Sizes
    SupportedPcmSizes = 0x0B,
    /// Supported PCM Formats
    SupportedPcmFormats = 0x0D,
    /// Input/Output Amplifier Capabilities
    AmplifierCap = 0x12,
    /// Connection List Length
    ConnectionListLength = 0x0E,
    /// Power State
    PowerState = 0x0F,
    /// Processing Coefficient
    ProcessingCoefficient = 0x10,
    /// GPIO Count
    GpioCount = 0x11,
}

/// HDA 코덱 정보
#[derive(Debug, Clone)]
pub struct HdaCodec {
    /// 코덱 ID
    pub codec_id: CodecId,
    /// Vendor ID
    pub vendor_id: u32,
    /// Revision ID
    pub revision_id: u32,
    /// Subsystem ID
    pub subsystem_id: u32,
    /// 노드 수
    pub node_count: u32,
    /// 오디오 함수 그룹 노드
    pub audio_function_group: Option<u16>,
    /// 출력 스트림 노드
    pub output_stream_nodes: Vec<u16>,
    /// 입력 스트림 노드
    pub input_stream_nodes: Vec<u16>,
}

impl HdaCodec {
    /// 새 코덱 생성
    pub fn new(codec_id: CodecId) -> Self {
        Self {
            codec_id,
            vendor_id: 0,
            revision_id: 0,
            subsystem_id: 0,
            node_count: 0,
            audio_function_group: None,
            output_stream_nodes: Vec::new(),
            input_stream_nodes: Vec::new(),
        }
    }
}

