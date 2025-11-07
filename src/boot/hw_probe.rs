//! Hardware probe and summary logging (CPU, PCI)

use crate::drivers::pci;
use core::arch::x86_64::__cpuid;
use core::sync::atomic::{AtomicU32, Ordering};

/// Log basic CPU vendor and family/model information
pub fn log_cpu_info() {
    let mut vendor = [0u32; 3];
    let mut brand_part = [0u32; 12];
    let (mut eax1, mut ebx1, mut ecx1, mut edx1) = (0u32, 0u32, 0u32, 0u32);
    unsafe {
        let v0 = __cpuid(0);
        vendor[0] = v0.ebx;
        vendor[1] = v0.edx;
        vendor[2] = v0.ecx;

        let l1 = __cpuid(1);
        eax1 = l1.eax; ebx1 = l1.ebx; ecx1 = l1.ecx; edx1 = l1.edx;

        let max_ext = __cpuid(0x8000_0000).eax;
        if max_ext >= 0x8000_0004 {
            for i in 0..3u32 {
                let r = __cpuid(0x8000_0002 + i);
                brand_part[(i as usize) * 4 + 0] = r.eax;
                brand_part[(i as usize) * 4 + 1] = r.ebx;
                brand_part[(i as usize) * 4 + 2] = r.ecx;
                brand_part[(i as usize) * 4 + 3] = r.edx;
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
static NET_CT: AtomicU32 = AtomicU32::new(0);
static STO_CT: AtomicU32 = AtomicU32::new(0);
static DSP_CT: AtomicU32 = AtomicU32::new(0);
static USB_CT: AtomicU32 = AtomicU32::new(0);

fn tally_pci(dev: &crate::drivers::pci::PciDevice) -> bool {
    match dev.class_code {
        crate::drivers::pci::PCI_CLASS_NETWORK => { NET_CT.fetch_add(1, Ordering::Relaxed); }
        crate::drivers::pci::PCI_CLASS_STORAGE => { STO_CT.fetch_add(1, Ordering::Relaxed); }
        crate::drivers::pci::PCI_CLASS_DISPLAY => { DSP_CT.fetch_add(1, Ordering::Relaxed); }
        0x0C => {
            if dev.subclass == 0x03 { USB_CT.fetch_add(1, Ordering::Relaxed); }
        }
        _ => {}
    }
    false
}

pub fn log_pci_summary() {
    NET_CT.store(0, Ordering::Relaxed);
    STO_CT.store(0, Ordering::Relaxed);
    DSP_CT.store(0, Ordering::Relaxed);
    USB_CT.store(0, Ordering::Relaxed);
    unsafe { pci::scan_pci_bus(tally_pci); }
    let net = NET_CT.load(Ordering::Relaxed);
    let storage = STO_CT.load(Ordering::Relaxed);
    let display = DSP_CT.load(Ordering::Relaxed);
    let usb_cnt = USB_CT.load(Ordering::Relaxed);
    crate::log_info!(
        "PCI: network={} storage={} display={} usb_ctl={}",
        net, storage, display, usb_cnt
    );
}


