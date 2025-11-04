//! Intel RAPL MSR reading (best-effort)

const MSR_PKG_ENERGY_STATUS: u32 = 0x611;

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

pub fn read_package_energy_status() -> Option<u32> {
    // Returns energy in arbitrary units; conversion requires energy unit MSR (0x606)
    unsafe {
        // This may #GP on unsupported CPUs; catch by using a volatile read pattern is not possible here
        // Callers should be prepared for None if an exception occurs; for now, assume available
        let val = read_msr(MSR_PKG_ENERGY_STATUS) as u32;
        Some(val)
    }
}


