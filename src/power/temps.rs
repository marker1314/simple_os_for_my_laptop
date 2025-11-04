//! CPU temperature reading (Intel best-effort)

const MSR_IA32_THERM_STATUS: u32 = 0x19C;

#[inline]
fn msr_supported() -> bool {
    let eax: u32 = 1; // CPUID leaf 1
    let mut eax_out: u32 = 0;
    let mut ebx_out: u32 = 0;
    let mut ecx_out: u32 = 0;
    let mut edx_out: u32 = 0;
    unsafe {
        core::arch::asm!(
            "cpuid",
            in("eax") eax,
            out("eax") eax_out,
            out("ebx") ebx_out,
            out("ecx") ecx_out,
            out("edx") edx_out,
            options(nostack, preserves_flags)
        );
    }
    // EDX bit 5 indicates MSR support
    (edx_out & (1 << 5)) != 0
}

#[inline]
unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Read approximate package temperature in Celsius assuming TjMax=100C
pub fn read_package_temperature_c() -> Option<u8> {
    if !msr_supported() { return None; }
    unsafe {
        let v = read_msr(MSR_IA32_THERM_STATUS);
        if (v & (1 << 31)) == 0 { return None; } // reading invalid
        let dts = ((v >> 16) & 0x7F) as u8; // delta to TjMax
        let tjmax = 100u8; // conservative default
        Some(tjmax.saturating_sub(dts))
    }
}


