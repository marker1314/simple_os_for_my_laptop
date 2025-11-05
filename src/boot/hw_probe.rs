//! Hardware probe and summary logging (CPU, PCI)

use crate::drivers::pci;

/// Log basic CPU vendor and family/model information
pub fn log_cpu_info() {
    let mut vendor = [0u32; 3];
    let mut brand_part = [0u32; 12];
    let (mut eax1, mut ebx1, mut ecx1, mut edx1) = (0u32, 0u32, 0u32, 0u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            in("eax") 0u32,
            lateout("eax") _,
            lateout("ebx") vendor[0],
            lateout("ecx") vendor[2],
            lateout("edx") vendor[1],
            options(nostack, preserves_flags)
        );
        core::arch::asm!(
            "cpuid",
            in("eax") 1u32,
            lateout("eax") eax1,
            lateout("ebx") ebx1,
            lateout("ecx") ecx1,
            lateout("edx") edx1,
            options(nostack, preserves_flags)
        );
        // Brand string if available (0x80000002..0x80000004)
        let mut max_ext: u32 = 0;
        core::arch::asm!(
            "cpuid",
            in("eax") 0x8000_0000u32,
            lateout("eax") max_ext,
            lateout("ebx") _,
            lateout("ecx") _,
            lateout("edx") _,
            options(nostack, preserves_flags)
        );
        if max_ext >= 0x8000_0004 {
            for i in 0..3u32 {
                let (mut a, mut b, mut c, mut d) = (0u32, 0u32, 0u32, 0u32);
                core::arch::asm!(
                    "cpuid",
                    in("eax") (0x8000_0002u32 + i),
                    lateout("eax") a,
                    lateout("ebx") b,
                    lateout("ecx") c,
                    lateout("edx") d,
                    options(nostack, preserves_flags)
                );
                brand_part[(i as usize) * 4 + 0] = a;
                brand_part[(i as usize) * 4 + 1] = b;
                brand_part[(i as usize) * 4 + 2] = c;
                brand_part[(i as usize) * 4 + 3] = d;
            }
        }
    }
    let vendor_bytes = unsafe { core::slice::from_raw_parts(vendor.as_ptr() as *const u8, 12) };
    let vendor_str = core::str::from_utf8(vendor_bytes).unwrap_or("unknown");
    let family = ((eax1 >> 8) & 0xF) + ((eax1 >> 20) & 0xFF);
    let model = ((eax1 >> 4) & 0xF) | (((eax1 >> 16) & 0xF) << 4);
    let stepping = eax1 & 0xF;
    let cores = ((ebx1 >> 16) & 0xFF) + 1; // rough estimate from CPUID leaf 1

    let brand_bytes = unsafe { core::slice::from_raw_parts(brand_part.as_ptr() as *const u8, 48) };
    let brand = core::str::from_utf8(brand_bytes).unwrap_or("").trim_matches('\0').trim();

    crate::log_info!(
        "CPU: {} fam {} model {} stepping {} cores ~{}",
        vendor_str,
        family,
        model,
        stepping,
        cores
    );
    if !brand.is_empty() { crate::log_info!("CPU brand: {}", brand); }
    let mm_entries = crate::boot::memory_map_len();
    crate::log_info!("Boot memory regions: {}", mm_entries);
}

/// Log a short PCI summary by class (network/storage/display/other)
pub fn log_pci_summary() {
    let mut net = 0u32;
    let mut storage = 0u32;
    let mut display = 0u32;
    let mut usb_cnt = 0u32;
    unsafe {
        pci::scan_pci_bus(|dev| {
            match dev.class_code {
                pci::PCI_CLASS_NETWORK => net += 1,
                pci::PCI_CLASS_STORAGE => storage += 1,
                pci::PCI_CLASS_DISPLAY => display += 1,
                0x0C => {
                    // Serial bus controllers; 0x0C03 is USB
                    if dev.subclass == 0x03 { usb_cnt += 1; }
                }
                _ => {}
            }
            false
        });
    }
    crate::log_info!(
        "PCI: network={} storage={} display={} usb_ctl={}",
        net, storage, display, usb_cnt
    );
}


