//! Minimal ACPI FADT (Fixed ACPI Description Table) parser
//!
//! Extracts PM1a/PM1b control register addresses and SLP_TYP values for S3.

use x86_64::PhysAddr;

#[derive(Clone, Copy, Debug, Default)]
pub struct FadtInfo {
    pub pm1a_cnt_blk: u32,
    pub pm1b_cnt_blk: u32,
    pub s3_sleep_type: Option<u16>,
}

impl FadtInfo {
    pub fn is_valid(&self) -> bool { self.pm1a_cnt_blk != 0 }
}

/// Parse a very small subset of FADT from a raw memory slice.
/// This expects the caller to map the table into memory already.
pub fn parse_fadt(data: &[u8]) -> Option<FadtInfo> {
    // FADT signature "FACP"
    if data.len() < 0x84 || &data[0..4] != b"FACP" { return None; }

    // Offsets are per ACPI 2.0+ (we keep it minimal and tolerant):
    // PM1a_CNT_BLK at offset 0x64 (32-bit)
    // PM1b_CNT_BLK at offset 0x68 (32-bit)
    // Sleep type values are not directly stored; typically via _Sx objects in DSDT.
    // For pragmatic use, we default S3 SLP_TYP to 5 if not discoverable.

    let pm1a = u32::from_le_bytes([data[0x64], data[0x65], data[0x66], data[0x67]]);
    let pm1b = u32::from_le_bytes([data[0x68], data[0x69], data[0x6A], data[0x6B]]);

    Some(FadtInfo { pm1a_cnt_blk: pm1a, pm1b_cnt_blk: pm1b, s3_sleep_type: Some(5) })
}


