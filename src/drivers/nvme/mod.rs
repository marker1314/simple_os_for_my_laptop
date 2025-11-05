//! NVMe (NVM Express) storage driver - skeleton
//!
//! Detect controller, map MMIO, and prepare Admin queue structures.
//! IO queue and PRP-based read/write to be implemented incrementally.

use crate::drivers::pci::PciDevice;
use crate::drivers::pci;
use core::ptr::{read_volatile, write_volatile};
use x86_64::{PhysAddr, VirtAddr};

#[derive(Debug)]
pub enum NvmeError {
    DeviceNotFound,
    InitFailed,
    IoError,
}

// NVMe MMIO registers (offsets)
const NVME_REG_CAP: usize = 0x0000; // Controller Capabilities
const NVME_REG_VS: usize = 0x0008;  // Version
const NVME_REG_CC: usize = 0x0014;  // Controller Configuration
const NVME_REG_CSTS: usize = 0x001C; // Controller Status
const NVME_REG_AQA: usize = 0x0024; // Admin Queue Attributes
const NVME_REG_ASQ: usize = 0x0028; // Admin Submission Queue Base (64b)
const NVME_REG_ACQ: usize = 0x0030; // Admin Completion Queue Base (64b)
const NVME_REG_DBS: usize = 0x1000; // Doorbells base

// CC bits
const CC_ENABLE: u32 = 1 << 0;
const CC_CSS_NVM: u32 = 0 << 4; // NVM Command Set
const CC_MPS_SHIFT: u32 = 7;    // Memory Page Size (2^(12+MPS))
const CC_IOCQES_SHIFT: u32 = 20; // IO Completion Queue Entry Size (2^(CQES))
const CC_IOSQES_SHIFT: u32 = 16; // IO Submission Queue Entry Size (2^(SQES))

// CSTS bits
const CSTS_RDY: u32 = 1 << 0;

// Admin opcodes
const OPC_IDENTIFY: u8 = 0x06;

// NVM opcodes
const OPC_READ: u8 = 0x02;
const OPC_WRITE: u8 = 0x01;

#[repr(C, packed)]
struct SqEntry {
    opc: u8,
    fuse_psdt: u8,
    cid: u16,
    nsid: u32,
    rsvd2: u64,
    mptr: u64,
    prp1: u64,
    prp2: u64,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
}

#[repr(C, packed)]
struct CqEntry {
    dw0: u32,
    rsvd: u32,
    sqhd: u16,
    sqid: u16,
    cid: u16,
    status_p: u16, // bit15:14 phase/status; bit0 phase tag
}

pub struct NvmeController {
    pci: PciDevice,
    bar0_phys: PhysAddr,
    mmio_base: Option<VirtAddr>,
    initialized: bool,
    // Admin queues
    asq: Option<PhysAddr>,
    acq: Option<PhysAddr>,
    asq_entries: u16,
    acq_entries: u16,
    asq_tail: u16,
    acq_head: u16,
    acq_phase: u16,
}

impl NvmeController {
    pub fn new(pci: PciDevice) -> Result<Self, NvmeError> {
        let bar0 = pci.bar0;
        if (bar0 & 0x01) != 0 { return Err(NvmeError::InitFailed); }
        Ok(Self {
            pci,
            bar0_phys: PhysAddr::new((bar0 & !0xF) as u64),
            mmio_base: None,
            initialized: false,
            asq: None,
            acq: None,
            asq_entries: 0,
            acq_entries: 0,
            asq_tail: 0,
            acq_head: 0,
            acq_phase: 1,
        })
    }

    unsafe fn map_mmio(&mut self) -> Result<VirtAddr, NvmeError> {
        if let Some(v) = self.mmio_base { return Ok(v); }
        let off = crate::memory::paging::get_physical_memory_offset(crate::boot::get_boot_info());
        let v = off + self.bar0_phys.as_u64();
        self.mmio_base = Some(v);
        Ok(v)
    }

    unsafe fn read_u32(&mut self, off: usize) -> Result<u32, NvmeError> {
        let base = self.map_mmio()?;
        let ptr = (base.as_u64() + off as u64) as *const u32;
        Ok(read_volatile(ptr))
    }
    unsafe fn write_u32(&mut self, off: usize, val: u32) -> Result<(), NvmeError> {
        let base = self.map_mmio()?;
        let ptr = (base.as_u64() + off as u64) as *mut u32;
        write_volatile(ptr, val);
        Ok(())
    }

    /// Initialize Admin queue (skeleton only)
    pub unsafe fn init(&mut self) -> Result<(), NvmeError> {
        // Enable bus master & memory space
        let cmd = self.pci.read_config_register(0x04);
        self.pci.write_config_register(0x04, cmd | 0x04 | 0x02);

        let cap_lo = self.read_u32(NVME_REG_CAP)?;
        let vs = self.read_u32(NVME_REG_VS)?;
        crate::log_info!("NVMe: MMIO={:#016X}, VS=0x{:08X}", self.bar0_phys.as_u64(), vs);

        // Determine page size min from CAP.MPSMIN (bits 51:48 in CAP hi)
        let cap_hi = self.read_u32(NVME_REG_CAP + 4)?;
        let mpsmin = ((cap_hi >> 16) & 0x0F) as u32; // CAP.MPSMIN
        let page_shift = 12 + mpsmin; // choose minimum supported

        // Disable controller if enabled
        let mut cc = self.read_u32(NVME_REG_CC)?;
        if (cc & CC_ENABLE) != 0 {
            cc &= !CC_ENABLE;
            self.write_u32(NVME_REG_CC, cc)?;
            // wait RDY=0
            let mut t = 100000;
            while (self.read_u32(NVME_REG_CSTS)? & CSTS_RDY) != 0 && t > 0 { t -= 1; core::hint::spin_loop(); }
        }

        // Allocate Admin SQ/CQ (one page each)
        let asq_frame = crate::memory::allocate_frame().ok_or(NvmeError::InitFailed)?;
        let acq_frame = crate::memory::allocate_frame().ok_or(NvmeError::InitFailed)?;
        self.asq = Some(asq_frame.start_address());
        self.acq = Some(acq_frame.start_address());
        self.asq_entries = 32; // 32 entries * 64B = 2KiB (fits in 4KiB page)
        self.acq_entries = 32; // 32 entries * 16B = 512B
        self.asq_tail = 0;
        self.acq_head = 0;
        self.acq_phase = 1;

        // Program AQA
        let aqa = ((self.acq_entries as u32 - 1) & 0xFFF) | ((((self.asq_entries as u32 - 1) & 0xFFF) << 16));
        self.write_u32(NVME_REG_AQA, aqa)?;
        // Program ASQ/ACQ (64-bit)
        let asq = self.asq.unwrap().as_u64();
        let acq = self.acq.unwrap().as_u64();
        self.write_u32(NVME_REG_ASQ, (asq & 0xFFFF_FFFF) as u32)?;
        self.write_u32(NVME_REG_ASQ + 4, (asq >> 32) as u32)?;
        self.write_u32(NVME_REG_ACQ, (acq & 0xFFFF_FFFF) as u32)?;
        self.write_u32(NVME_REG_ACQ + 4, (acq >> 32) as u32)?;

        // Configure CC (enable, page size, entry sizes 64B/16B)
        let iosqes = 6; // 2^6 = 64 bytes
        let iocqes = 4; // 2^4 = 16 bytes
        cc = 0;
        cc |= CC_CSS_NVM;
        cc |= (page_shift - 12) << CC_MPS_SHIFT;
        cc |= (iosqes as u32) << CC_IOSQES_SHIFT;
        cc |= (iocqes as u32) << CC_IOCQES_SHIFT;
        cc |= CC_ENABLE;
        self.write_u32(NVME_REG_CC, cc)?;
        // Wait RDY=1
        let mut t = 100000;
        while (self.read_u32(NVME_REG_CSTS)? & CSTS_RDY) == 0 && t > 0 { t -= 1; core::hint::spin_loop(); }
        if t == 0 { return Err(NvmeError::InitFailed); }

        self.initialized = true;
        Ok(())
    }

    /// Submit an admin command and wait for completion (polled)
    unsafe fn admin_submit_and_wait(&mut self, sqe: &SqEntry) -> Result<CqEntry, NvmeError> {
        let phys_off = crate::memory::paging::get_physical_memory_offset(crate::boot::get_boot_info());
        // ASQ virtual mapping
        let asq_va = (phys_off + self.asq.unwrap().as_u64()).as_mut_ptr::<SqEntry>();
        let acq_va = (phys_off + self.acq.unwrap().as_u64()).as_mut_ptr::<CqEntry>();
        // Write SQ entry at tail
        let tail = self.asq_tail as usize % self.asq_entries as usize;
        core::ptr::write_volatile(asq_va.add(tail), *sqe);
        // Ring SQ tail doorbell (admin qid=0)
        self.write_u32(NVME_REG_DBS + (0 * 2 * 4), (self.asq_tail as u32).wrapping_add(1))?;
        self.asq_tail = self.asq_tail.wrapping_add(1);

        // Poll completion
        let mut timeout = 1000000;
        loop {
            let head = self.acq_head as usize % self.acq_entries as usize;
            let cqe = core::ptr::read_volatile(acq_va.add(head));
            let phase = (cqe.status_p & 1) as u16;
            if phase == self.acq_phase {
                // advance head and toggle phase when wrap
                self.acq_head = self.acq_head.wrapping_add(1);
                if self.acq_head as usize % self.acq_entries as usize == 0 { self.acq_phase ^= 1; }
                // Ring CQ head doorbell
                self.write_u32(NVME_REG_DBS + ((0 * 2 + 1) * 4), self.acq_head as u32)?;
                return Ok(cqe);
            }
            timeout -= 1;
            if timeout == 0 { return Err(NvmeError::IoError); }
            core::hint::spin_loop();
        }
    }

    /// Issue Identify Controller (CNS=0) into provided buffer
    pub unsafe fn identify_controller(&mut self, dst_phys: PhysAddr) -> Result<(), NvmeError> {
        if !self.initialized { return Err(NvmeError::InitFailed); }
        let sqe = SqEntry {
            opc: OPC_IDENTIFY,
            fuse_psdt: 0,
            cid: 1,
            nsid: 0,
            rsvd2: 0,
            mptr: 0,
            prp1: dst_phys.as_u64(),
            prp2: 0,
            cdw10: 0, // CNS=0: identify controller
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        let cqe = self.admin_submit_and_wait(&sqe)?;
        let status = (cqe.status_p >> 1) & 0x7FFF;
        if status != 0 { return Err(NvmeError::IoError); }
        Ok(())
    }

    /// Read one LBA into PRP1 buffer for NSID=1
    pub unsafe fn read_lba(&mut self, nsid: u32, lba: u64, blocks: u16, dst_phys: PhysAddr) -> Result<(), NvmeError> {
        let sqe = SqEntry {
            opc: OPC_READ,
            fuse_psdt: 0,
            cid: 2,
            nsid,
            rsvd2: 0,
            mptr: 0,
            prp1: dst_phys.as_u64(),
            prp2: 0,
            cdw10: (lba & 0xFFFF_FFFF) as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: ((blocks as u32 - 1) & 0xFFFF),
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        let cqe = self.admin_submit_and_wait(&sqe)?;
        let status = (cqe.status_p >> 1) & 0x7FFF;
        if status != 0 { return Err(NvmeError::IoError); }
        Ok(())
    }

    /// Write one LBA from PRP1 buffer for NSID=1
    pub unsafe fn write_lba(&mut self, nsid: u32, lba: u64, blocks: u16, src_phys: PhysAddr) -> Result<(), NvmeError> {
        let sqe = SqEntry {
            opc: OPC_WRITE,
            fuse_psdt: 0,
            cid: 3,
            nsid,
            rsvd2: 0,
            mptr: 0,
            prp1: src_phys.as_u64(),
            prp2: 0,
            cdw10: (lba & 0xFFFF_FFFF) as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: ((blocks as u32 - 1) & 0xFFFF),
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        let cqe = self.admin_submit_and_wait(&sqe)?;
        let status = (cqe.status_p >> 1) & 0x7FFF;
        if status != 0 { return Err(NvmeError::IoError); }
        Ok(())
    }
}

static mut NVME: Option<NvmeController> = None;

/// Detect and initialize the first NVMe controller (if present)
pub unsafe fn init() -> Result<(), NvmeError> {
    if let Some(pci_dev) = find_nvme_controller() {
        let mut ctrl = NvmeController::new(pci_dev)?;
        ctrl.init()?;
        // Try Identify Controller to verify path
        let frame = crate::memory::allocate_frame().ok_or(NvmeError::InitFailed)?;
        let phys = frame.start_address();
        if let Err(_e) = ctrl.identify_controller(phys) {
            crate::log_warn!("NVMe: Identify controller failed (continuing)");
        }
        NVME = Some(ctrl);
        Ok(())
    } else {
        Err(NvmeError::DeviceNotFound)
    }
}

unsafe fn find_nvme_controller() -> Option<PciDevice> {
    let mut found: Option<PciDevice> = None;
    pci::scan_pci_bus(|d| {
        // Class 0x01 = Mass Storage, Subclass 0x08 = NVM, ProgIF 0x02 = NVMe
        if d.class_code == 0x01 && d.subclass == 0x08 && d.prog_if == 0x02 {
            crate::log_info!("Found NVMe controller {:04X}:{:04X}", d.vendor_id, d.device_id);
            found = Some(*d);
            true
        } else { false }
    });
    found
}

// Block device adapter to existing ATA trait
pub struct NvmeBlockDevice;

impl crate::drivers::ata::BlockDevice for NvmeBlockDevice {
    fn block_size(&self) -> usize { 512 }
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<usize, crate::drivers::ata::BlockDeviceError> {
        if buf.len() < 512 { return Err(crate::drivers::ata::BlockDeviceError::InvalidBuffer); }
        unsafe {
            if let Some(ctrl) = NVME.as_mut() {
                // allocate one page temp buffer and read
                let frame = crate::memory::allocate_frame().ok_or(crate::drivers::ata::BlockDeviceError::ReadError)?;
                let phys = frame.start_address();
                ctrl.read_lba(1, block, 1, phys).map_err(|_| crate::drivers::ata::BlockDeviceError::ReadError)?;
                let off = crate::memory::paging::get_physical_memory_offset(crate::boot::get_boot_info());
                let src = (off + phys.as_u64()).as_ptr::<u8>();
                for i in 0..512 { buf[i] = unsafe { core::ptr::read_volatile(src.add(i)) }; }
                Ok(512)
            } else {
                Err(crate::drivers::ata::BlockDeviceError::NotReady)
            }
        }
    }
    fn write_block(&mut self, _block: u64, _buf: &[u8]) -> Result<usize, crate::drivers::ata::BlockDeviceError> {
        if _buf.len() < 512 { return Err(crate::drivers::ata::BlockDeviceError::InvalidBuffer); }
        unsafe {
            if let Some(ctrl) = NVME.as_mut() {
                let frame = crate::memory::allocate_frame().ok_or(crate::drivers::ata::BlockDeviceError::WriteError)?;
                let phys = frame.start_address();
                let off = crate::memory::paging::get_physical_memory_offset(crate::boot::get_boot_info());
                let dst = (off + phys.as_u64()).as_mut_ptr::<u8>();
                for i in 0..512 { core::ptr::write_volatile(dst.add(i), _buf[i]); }
                ctrl.write_lba(1, _block, 1, phys).map_err(|_| crate::drivers::ata::BlockDeviceError::WriteError)?;
                Ok(512)
            } else {
                Err(crate::drivers::ata::BlockDeviceError::NotReady)
            }
        }
    }
    fn num_blocks(&self) -> u64 { 0 }
}


