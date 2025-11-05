//! Crash capture with symbolization support (best-effort)

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrashDump {
    pub magic: u32,
    pub reason: u32,      // 1=panic, 2=exception
    pub rip: u64,
    pub code: u64,
    pub rbp: u64,         // Stack frame pointer for backtrace
    pub stack_trace: [u64; 8], // Up to 8 stack frames
    pub stack_trace_len: u8,
}

impl CrashDump {
    /// Create a new crash dump
    pub const fn new() -> Self {
        Self {
            magic: 0,
            reason: 0,
            rip: 0,
            code: 0,
            rbp: 0,
            stack_trace: [0; 8],
            stack_trace_len: 0,
        }
    }

    /// Collect stack trace from current RBP
    /// This walks the stack frame chain to collect return addresses
    pub unsafe fn collect_stack_trace(&mut self) {
        self.stack_trace_len = 0;
        let mut frame_ptr = self.rbp;
        
        // Walk stack frames (limited to 8 for safety)
        for _i in 0..8 {
            if frame_ptr == 0 || self.stack_trace_len >= 8 {
                break;
            }
            
            // Read return address from stack frame
            // Stack frame layout: [old_rbp][return_addr]
            let ret_addr_ptr = (frame_ptr + 8) as *const u64;
            let ret_addr = core::ptr::read_unaligned(ret_addr_ptr);
            
            // Basic sanity check: return address should be in kernel space
            if ret_addr > 0xFFFF800000000000 && ret_addr < 0xFFFFFFFFFFFFFFFF {
                self.stack_trace[self.stack_trace_len as usize] = ret_addr;
                self.stack_trace_len += 1;
                
                // Read next frame pointer
                let next_frame_ptr = frame_ptr as *const u64;
                let next_frame = core::ptr::read_unaligned(next_frame_ptr);
                
                // Check for cycle or invalid frame
                if next_frame <= frame_ptr || next_frame == 0 {
                    break;
                }
                frame_ptr = next_frame;
            } else {
                break;
            }
        }
    }
}

#[link_section = ".noinit"]
static mut LAST_CRASH: CrashDump = CrashDump::new();

/// Record a panic with RIP capture
pub fn record_panic() {
    unsafe {
        // Try to capture RIP from RSP (best-effort)
        let rip: u64;
        #[cfg(target_arch = "x86_64")]
        {
            // Read RIP from stack (if available)
            core::arch::asm!("mov {}, [rsp]", out(reg) rip, options(nostack, preserves_flags));
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            rip = 0;
        }
        
        let rbp: u64;
        #[cfg(target_arch = "x86_64")]
        {
            core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nostack, preserves_flags));
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            rbp = 0;
        }
        
        LAST_CRASH = CrashDump {
            magic: 0x43525348,
            reason: 1,
            rip,
            code: 0,
            rbp,
            stack_trace: [0; 8],
            stack_trace_len: 0,
        };
        
        // Collect stack trace
        LAST_CRASH.collect_stack_trace();
    }
}

/// Record an exception with RIP and error code
pub fn record_exception(rip: u64, code: u64) {
    unsafe {
        let rbp: u64;
        #[cfg(target_arch = "x86_64")]
        {
            core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nostack, preserves_flags));
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            rbp = 0;
        }
        
        LAST_CRASH = CrashDump {
            magic: 0x43525348,
            reason: 2,
            rip,
            code,
            rbp,
            stack_trace: [0; 8],
            stack_trace_len: 0,
        };
        
        // Collect stack trace
        LAST_CRASH.collect_stack_trace();
    }
}

/// Take and clear the crash dump
pub fn take() -> Option<CrashDump> {
    unsafe {
        if LAST_CRASH.magic == 0x43525348 {
            let dump = LAST_CRASH;
            LAST_CRASH.magic = 0; // clear after read
            Some(dump)
        } else {
            None
        }
    }
}

/// Export crash dump to CSV format for analysis
pub fn export_crash_dump_csv(dump: &CrashDump) {
    let reason_str = match dump.reason {
        1 => "PANIC",
        2 => "EXCEPTION",
        _ => "UNKNOWN",
    };
    
    crate::serial_println!("crash_dump.csv");
    crate::serial_println!("reason,code,rip,rbp,stack_trace_len");
    crate::serial_println!("{},{},0x{:016x},0x{:016x},{}", reason_str, dump.code, dump.rip, dump.rbp, dump.stack_trace_len);
    
    if dump.stack_trace_len > 0 {
        crate::serial_println!("\nstack_trace.csv");
        crate::serial_println!("frame,address");
        for i in 0..dump.stack_trace_len as usize {
            let addr = dump.stack_trace[i];
            if addr != 0 {
                crate::serial_println!("{},0x{:016x}", i, addr);
            }
        }
    }
}

/// Print a symbolized crash dump
pub fn print_crash_dump(dump: &CrashDump) {
    let reason_str = match dump.reason {
        1 => "PANIC",
        2 => "EXCEPTION",
        _ => "UNKNOWN",
    };
    
    let exception_name = match dump.code {
        0x00 => "Divide Error",
        0x01 => "Debug",
        0x02 => "Non-Maskable Interrupt",
        0x03 => "Breakpoint",
        0x04 => "Overflow",
        0x05 => "Bound Range Exceeded",
        0x06 => "Invalid Opcode",
        0x07 => "Device Not Available",
        0x08 => "Double Fault",
        0x09 => "Coprocessor Segment Overrun",
        0x0A => "Invalid TSS",
        0x0B => "Segment Not Present",
        0x0C => "Stack Segment Fault",
        0x0D => "General Protection Fault",
        0x0E => "Page Fault",
        0x10 => "x87 Floating Point",
        0x11 => "Alignment Check",
        0x12 => "Machine Check",
        0x13 => "SIMD Floating Point",
        0x14 => "Virtualization",
        0x1E => "Security",
        _ => "Unknown Exception",
    };
    
    crate::log_error!("=== Crash Dump ===");
    crate::log_error!("Reason: {} (code: 0x{:x})", reason_str, dump.code);
    if dump.reason == 2 {
        crate::log_error!("Exception: {}", exception_name);
    }
    crate::log_error!("RIP: 0x{:016x}", dump.rip);
    crate::log_error!("RBP: 0x{:016x}", dump.rbp);
    
    // Try to identify function from RIP (simplified symbol resolution)
    // In a real implementation, this would use a symbol table
    let rip_addr = dump.rip;
    let kernel_base = 0xFFFF800000000000u64;
    if rip_addr >= kernel_base && rip_addr < 0xFFFFFFFFFFFFFFFFu64 {
        let offset = rip_addr - kernel_base;
        crate::log_error!("RIP offset: 0x{:016x} (from kernel base)", offset);
        crate::log_error!("Note: Use objdump or addr2line to resolve symbols:");
        crate::log_error!("  addr2line -e target/x86_64-unknown-none/debug/simple_os 0x{:016x}", offset);
    }
    
    if dump.stack_trace_len > 0 {
        crate::log_error!("Stack Trace ({} frames):", dump.stack_trace_len);
        for i in 0..dump.stack_trace_len as usize {
            let addr = dump.stack_trace[i];
            if addr != 0 {
                let offset = if addr >= kernel_base {
                    addr - kernel_base
                } else {
                    0
                };
                crate::log_error!("  #{}: 0x{:016x} (offset: 0x{:016x})", i, addr, offset);
            }
        }
        crate::log_error!("To symbolize: addr2line -e target/x86_64-unknown-none/debug/simple_os -a -f -C <addresses>");
    }
    
    // Additional context
    crate::log_error!("Timestamp: {}ms", crate::drivers::timer::get_milliseconds());
    crate::log_error!("==================");
}



