# Rust-Based Laptop Operating System

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-x86__64-lightgrey.svg)](https://en.wikipedia.org/wiki/X86-64)

A feature-rich laptop operating system kernel written in Rust with comprehensive driver support and a GUI desktop. This is a completely independent implementation from the Linux kernel. The project prioritizes modular architecture, broad hardware support, and practical power management.

## 📋 Table of Contents

- [Features](#-features)
- [Goals](#-goals)
- [Getting Started](#-getting-started)
- [Requirements](#-requirements)
- [Build and Run](#-build-and-run)
- [Project Structure](#-project-structure)
- [Development Roadmap](#-development-roadmap)
- [Contributing](#-contributing)
- [License](#-license)
- [References](#-references)

## ✨ Features

- **Memory Safety**: Minimizes kernel-level bugs through Rust's type system and memory safety guarantees
- **Power-Aware Design**: ACPI-based power management framework and dynamic CPU clock scaling interface (P-State/C-State control in progress)
- **Independent Implementation**: Completely independent kernel design from Linux
- **Modular Architecture**: Extensible and maintainable structure
- **no_std Environment**: Lightweight kernel running without the standard library
- **Storage Support**: Full ATA/SATA disk driver with read/write capabilities
- **Interactive Shell**: Command-line interface with disk management commands
- **Network Stack**: Complete TCP/IP stack with RTL8139 Ethernet driver support
- **GUI System**: VESA framebuffer-based graphical user interface with window management
- **Input Devices**: PS/2 keyboard and mouse support with event handling
- **Touchpad Support**: I2C-HID touchpad driver (ELAN708:00 04F3:30A0 and compatible devices)
- **GUI Applications**: Calculator, text editor, file manager, system monitor, and terminal emulator
- **Multi-core Support**: SMP (Symmetric Multiprocessing) with APIC and load balancing

## 🎯 Goals

### Functional Goals
- Independent operating system kernel implementation
- Comprehensive driver support (keyboard, mouse, display, storage, network)
- Power management system (ACPI parsing, dynamic scaling)
- Interactive Shell and GUI system with graphical capabilities
- Filesystem support (FAT32)

### Non-Functional Goals
- **Boot Time**: Target under 5 seconds (hardware dependent)
- **Idle Power Consumption**: Target under 5W on supported hardware (measurement and tuning ongoing)
- **Memory Usage**: Minimum 64MB RAM (Recommended: 512MB or more)
- **Stability**: Long-term operation without kernel panics

## 📏 Project Scale and Scope

- Code size: 13,000+ lines across ~75 Rust source files
- Subsystems: memory management, scheduler, filesystem (FAT32), network stack (TCP/IP), GUI with applications, SMP, drivers (ATA, PCI, RTL8139, PS/2, I2C-HID), ACPI-based power framework
- Architecture: modular, componentized design intended for experimentation and learning

## 🚀 Getting Started

### Requirements

#### Essential Tools
- **Rust (nightly)**: `rustup install nightly`
- **bootimage**: `cargo install bootimage`
- **QEMU**: For testing in a virtualized environment

#### Windows
```powershell
# Install Rust
winget install Rustlang.Rustup
rustup install nightly
rustup default nightly

# Essential components
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none

# Install bootimage
cargo install bootimage

# Install QEMU (optional)
winget install SoftwareFreedomConservancy.QEMU
```

#### Linux
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install nightly
rustup default nightly

# Essential components
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none

# Install bootimage
cargo install bootimage

# Install QEMU and other tools
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    nasm \
    qemu-system-x86 \
    qemu-utils \
    ovmf
```

### Build and Run

#### 1. Clone Repository
```bash
git clone https://github.com/yourusername/simple_os_for_my_laptop.git
cd simple_os_for_my_laptop
```

#### 2. Build Kernel
```bash
# Debug build
cargo build

# Generate boot image
cargo bootimage

# Release build (optimized)
cargo build --release
cargo bootimage --release
```

#### 3. Run in QEMU
```bash
# Basic execution
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio \
    -display none

# Or use script (Linux/macOS)
./run.sh

# On Windows
.\run.bat
```

#### 4. Debug Mode
```bash
# Run QEMU in GDB server mode
qemu-system-x86_64 \
    -s -S \
    -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-simple_os.bin \
    -serial stdio

# Connect GDB from separate terminal
rust-gdb target/x86_64-unknown-none/debug/simple_os
(gdb) target remote :1234
```

## 📁 Project Structure

```
simple_os_for_my_laptop/
├── Cargo.toml              # Project configuration and dependencies
├── Cargo.lock              # Dependency version lock
├── README.md               # Project introduction (this file)
├── roadmap.md              # Detailed development roadmap
├── LICENSE                 # License file
├── .cargo/
│   └── config.toml         # Cargo configuration
├── src/
│   ├── main.rs            # Kernel entry point
│   ├── lib.rs             # Library root
│   ├── boot/              # Bootloader interface
│   ├── memory/            # Memory management
│   │   ├── mod.rs         # Memory management module integration
│   │   ├── map.rs         # Memory map parsing and classification
│   │   ├── frame.rs       # Physical memory frame allocator
│   │   ├── paging.rs      # Virtual memory and page table management
│   │   └── heap.rs        # Heap allocator (linked_list_allocator)
│   ├── scheduler/         # Process/thread scheduler
│   │   ├── mod.rs         # Scheduler module integration
│   │   ├── thread.rs       # Thread structure and context management
│   │   ├── round_robin.rs  # Round-Robin scheduler implementation
│   │   └── context_switch.rs # Context switching implementation
│   ├── power/             # Power management
│   │   ├── mod.rs         # Power management module integration
│   │   ├── manager.rs     # Power manager
│   │   ├── acpi.rs        # ACPI parsing
│   │   ├── scaling.rs     # Dynamic scaling
│   │   └── policy.rs      # Power policy management
│   ├── drivers/           # Hardware drivers
│   │   ├── keyboard.rs    # Keyboard driver
│   │   ├── mouse.rs       # PS/2 mouse driver
│   │   ├── i2c.rs         # I2C bus controller driver
│   │   ├── i2c_hid.rs     # I2C-HID protocol implementation
│   │   ├── touchpad.rs    # ELAN I2C-HID touchpad driver
│   │   ├── vga.rs         # VGA display
│   │   ├── framebuffer.rs # VESA framebuffer driver
│   │   ├── font.rs        # Font rendering
│   │   ├── timer.rs       # Timer
│   │   ├── serial.rs      # Serial port
│   │   ├── ata.rs         # ATA/SATA storage driver
│   │   ├── pci.rs         # PCI bus driver
│   │   └── rtl8139.rs     # RTL8139 Ethernet driver
│   ├── interrupts/        # Interrupt handlers
│   ├── logging.rs         # Logging system
│   ├── sync/              # Synchronization primitives
│   ├── syscall/           # System call interface
│   │   ├── mod.rs         # System call module integration
│   │   ├── numbers.rs     # System call number definitions
│   │   ├── handler.rs     # System call handler
│   │   ├── dispatcher.rs  # System call dispatcher
│   │   └── implementations.rs # System call implementations
│   ├── shell/             # Shell interface
│   │   ├── mod.rs         # Shell main logic
│   │   └── command.rs     # Command processing
│   ├── fs/                # Filesystem interface
│   │   ├── mod.rs         # Filesystem module integration
│   │   ├── vfs.rs         # Virtual Filesystem (VFS) interface
│   │   └── fat32.rs       # FAT32 filesystem implementation
│   ├── net/               # Network stack
│   │   ├── mod.rs         # Network module integration
│   │   ├── ethernet.rs    # Ethernet driver interface
│   │   ├── driver.rs      # Network driver management
│   │   ├── ip.rs          # IP (IPv4) protocol
│   │   ├── arp.rs         # ARP protocol
│   │   ├── icmp.rs        # ICMP protocol
│   │   ├── udp.rs         # UDP protocol
│   │   ├── tcp.rs         # TCP protocol
│   │   └── ethernet_frame.rs # Ethernet frame processing
│   └── gui/               # GUI system
│       ├── mod.rs         # GUI module integration
│       ├── window.rs      # Window management
│       ├── widget.rs      # GUI widgets (Button, TextBox)
│       ├── compositor.rs  # Display compositor
│       └── applications/  # GUI applications
│           ├── mod.rs     # Applications module
│           ├── calculator.rs      # Calculator app
│           ├── text_editor.rs     # Text editor app
│           ├── file_manager.rs    # File manager app
│           ├── system_monitor.rs  # System monitor app
│           └── terminal.rs        # Terminal emulator app
├── tests/                 # Integration tests
├── docs/                  # Additional documentation
└── scripts/               # Build/run scripts
    ├── run.sh             # Linux/macOS run script
    └── run.bat            # Windows run script
```

## 🔋 Power Management Status

Current status:
- ACPI parsing framework and power policy interface: implemented
- CPU dynamic scaling (P-State/C-State): interfaces present, MSR/MWAIT wiring in progress
- Device power management (disk/network/display): planned
- Power monitoring/estimation: planned

See the roadmap for details.

## 🗺️ Development Roadmap

See [roadmap.md](roadmap.md) for detailed development roadmap.

### Current Status

**Phase 1: Overall Direction and Architecture Design (Completed)**
- [x] Project structure design
- [x] Architecture documentation (`docs/architecture.md`)
- [x] Project requirements documentation (`docs/requirements.md`)
- [x] Basic project structure creation
- [x] Cargo.toml and configuration file creation
- [x] Kernel module structure creation

**Phase 2: Rust OS Development Environment Setup (Completed)**
- [x] Rust toolchain installation and setup script
- [x] Cross-compilation environment setup
- [x] Debugging environment setup script
- [x] Basic logging system implementation
- [x] QEMU test script creation (Windows/Linux)
- [x] Development environment setup guide

**Phase 3: Bootloader and Kernel Initialization (Completed)**
- [x] Bootloader integration (bootloader crate)
- [x] Boot information parsing and storage
- [x] IDT (Interrupt Descriptor Table) implementation
- [x] PIC (Programmable Interrupt Controller) remapping
- [x] Exception handler implementation (all x86_64 exceptions)
- [x] Interrupt activation

**Phase 4: Memory Management System Implementation (Completed)**
- [x] Initial memory map parsing and classification
- [x] Physical memory frame allocator implementation (4KB page units)
- [x] Virtual memory management and page table access
- [x] Heap allocator initialization (100KB heap area)
- [x] Memory management system integration initialization
- [x] bootloader_api 0.11.12 compatibility

**Phase 5: Basic Driver Implementation (Completed)**
- [x] Serial port driver (logging and debugging)
- [x] Timer driver (PIT-based, millisecond time tracking)
- [x] Keyboard driver (PS/2 keyboard interrupt handling)
- [x] VGA text mode driver (80x25 text output)
- [x] ATA/SATA storage driver (block device interface, PIO mode, read/write operations)
- [x] Driver initialization and integration

**Phase 6: Scheduler Implementation (Completed)**
- [x] Thread structure and context management (Thread, ThreadContext)
- [x] Round-Robin scheduler implementation
- [x] Context switching mechanism
- [x] Thread state management (Ready, Running, Blocked, Terminated)
- [x] Time quantum-based scheduling
- [x] Scheduler initialization and integration

**Phase 7: System Call Interface Implementation (Completed)**
- [x] System call handler implementation (interrupt 0x80)
- [x] System call dispatcher implementation
- [x] Basic system call implementation (Exit, Write, Read, Yield, Sleep, GetTime, GetPid)
- [x] System call error handling mechanism
- [x] System call handler initialization and integration

**Phase 8: Basic Shell Implementation (Completed)**
- [x] Shell structure and main loop implementation
- [x] Keyboard input processing (Enter, Backspace, Tab support)
- [x] Command parsing and execution system
- [x] Basic command implementation (help, clear, echo, uptime, exit)
- [x] Disk management commands (disk, read, write)
- [x] VGA text mode output integration
- [x] Shell initialization and kernel integration

**Phase 9: Filesystem Support Implementation (Completed)**
- [x] Virtual Filesystem (VFS) interface implementation
- [x] ATA block device driver interface implementation
- [x] ATA driver read/write operations fully implemented
- [x] FAT32 filesystem basic structure implementation
- [x] FAT32 read functionality completion
- [x] FAT32 write functionality implementation (file creation, directory creation, file writing)
- [x] Filesystem mount and integration

**Phase 10: Power Management System Implementation (Completed)**
- [x] Power manager structure and initialization system implementation
- [x] ACPI RSDP address extraction and parsing foundation
- [x] ACPI table parsing module implementation (RSDP, RSDT/XSDT, FADT, etc.)
- [x] CPU clock scaling module implementation (P-State control)
- [x] CPU idle state management module implementation (C-State control)
- [x] Power policy management system implementation
- [x] Power management system kernel integration

**Phase 11: PCI Driver and Network Module Implementation (Completed)**
- [x] PCI bus scan and device discovery module implementation
- [x] PCI configuration space read/write functionality implementation
- [x] Network device discovery functionality implementation
- [x] Ethernet driver interface definition (EthernetDriver trait)
- [x] Network driver manager implementation
- [x] MAC address and packet buffer structure implementation
- [x] Network module kernel integration

**Phase 12: Actual Network Driver Implementation (Completed)**
- [x] RTL8139 Ethernet driver implementation (register definition, initialization, interrupt handling)
- [x] RTL8139 driver integration into network driver manager
- [x] PCI interrupt line read functionality addition
- [x] Network interrupt handler registration and activation
- [x] Network module kernel initialization integration
- [x] Complete packet transmission/reception implementation (TX/RX buffer management, physical memory allocation, packet processing)

**Phase 13: Network Protocol Stack Implementation (Completed)**
- [x] IP (IPv4) protocol implementation (header structure, packet parsing/generation, checksum calculation)
- [x] ARP (Address Resolution Protocol) implementation (MAC address resolution, ARP table management)
- [x] ICMP (Internet Control Message Protocol) implementation (Echo Request/Reply, ping support)
- [x] UDP protocol implementation (header structure, packet transmission/reception, port management)
- [x] TCP protocol basic structure implementation (header structure, packet processing)
- [x] Ethernet frame processing module implementation
- [x] Network stack module integration and kernel integration

**Phase 14: GUI System Implementation (Completed)**
- [x] VESA framebuffer driver implementation (pixel manipulation, color support)
- [x] Basic graphics primitives (rectangle, circle, line drawing)
- [x] Font rendering module implementation
- [x] PS/2 mouse driver implementation (event handling, position tracking)
- [x] Window management system implementation
- [x] GUI widget system (Button, TextBox)
- [x] Display compositor implementation
- [x] GUI system kernel integration

**Phase 15: Advanced GUI Applications Implementation (Completed)**
- [x] Calculator application (functional calculator with basic operations)
- [x] Text editor application (multi-line editing with file support)
- [x] File manager application (directory navigation, file browsing)
- [x] System monitor application (CPU/memory usage, process list)
- [x] Terminal emulator (GUI-based terminal with command execution)

**Phase 16: Multi-core Support (SMP) (Completed)**
- [x] APIC (Advanced Programmable Interrupt Controller) driver
- [x] Local APIC and I/O APIC initialization
- [x] IPI (Inter-Processor Interrupt) mechanism
- [x] CPU information management and detection
- [x] Load balancer for multi-core task distribution
- [x] Support for Round-Robin and Least-Loaded scheduling strategies

**Phase 17: Enhanced Filesystem Features (Completed)**
- [x] Path processing utilities with normalization and validation
- [x] Block cache system with LRU replacement policy (up to 256 blocks)
- [x] File and directory deletion (remove) functionality
- [x] File and directory renaming/moving (rename) functionality
- [x] Enhanced FAT32 operations (split_path, is_directory_empty, free_cluster_chain)
- [x] Cache statistics and performance monitoring

**Phase 18: Application Launcher and Desktop Environment (Completed)**
- [x] Desktop environment structure (desktop manager, taskbar)
- [x] Application launcher with icon grid layout
- [x] Taskbar with start button and system tray
- [x] Desktop manager integration with GUI applications
- [x] Mouse-driven application launching
- [x] 60 FPS rendering loop in desktop mode
- [x] Automatic window offset for multiple apps

**Phase 19: I2C and Touchpad Support (Completed)**
- [x] I2C bus controller driver (AMD FCH I2C)
- [x] I2C-HID protocol layer implementation
- [x] ELAN touchpad driver (ELAN708:00 04F3:30A0)
- [x] ACPI-based I2C device detection
- [x] Touchpad event to MouseEvent conversion
- [x] Kernel integration with dual input support (PS/2 + I2C touchpad)

### Planned Features

**Mid-term Goals**
- [x] Scheduler implementation
- [x] System call interface
- [x] Basic Shell implementation

**Long-term Goals**
- [x] Power management system (ACPI parsing foundation) - Completed
- [x] Dynamic power scaling - Completed
- [x] Filesystem (FAT32) - Read/write functionality completed
- [x] ATA/SATA storage driver - Fully implemented with read/write operations
- [x] PCI driver and network module - Completed
- [x] Actual network driver implementation (RTL8139) - Completed
- [x] Network protocol stack (IP, TCP/UDP, ARP, ICMP) - Completed
- [x] Interactive shell with disk management - Completed
- [x] GUI system (framebuffer, window manager, widgets) - Completed
- [x] Mouse driver (PS/2 mouse support) - Completed
- [x] Advanced GUI applications (Calculator, Text Editor, File Manager, System Monitor, Terminal) - Completed
- [x] Multi-core support (SMP with APIC and load balancing) - Completed
- [x] Enhanced filesystem features (Path processing, block cache, delete/rename) - Completed
- [x] Application launcher and desktop environment - Completed
- [x] I2C-HID touchpad support (ELAN708 and compatible devices) - Completed

## 🛠️ Technology Stack

### Core Technologies
- **Language**: Rust (nightly)
- **Architecture**: x86_64
- **Boot Protocol**: UEFI (BIOS legacy support planned)
- **Environment**: `no_std` (no standard library)

### Key Crates
- `bootloader_api` (0.11.12) - Bootloader integration and boot information
- `x86_64` (0.14) - x86_64 architecture support and page table management
- `volatile` (0.4) - Hardware register access
- `spin` (0.9) - Spinlock implementation
- `uart_16550` (0.2) - Serial port communication
- `linked_list_allocator` (0.10) - Heap allocator implementation

### Future Additions
- `acpi` - ACPI table parsing (currently implementing directly)
- `embedded-graphics` - GUI framework
- `smoltcp` - Network stack (basic structure implementation completed)

## 🤝 Contributing

Contributions are welcome! If you'd like to contribute to this project:

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style
- Use `rustfmt` for code formatting
- Resolve `clippy` warnings
- Write meaningful commit messages

### Issue Reports
If you find bugs or have feature suggestions, please register them in [Issues](https://github.com/yourusername/simple_os_for_my_laptop/issues).

## 📝 License

The license for this project has not yet been determined. A LICENSE file will be added after project policy decisions are made.

## 📚 References

### Learning Resources
- [Writing an OS in Rust](https://os.phil-opp.com/) - Rust OS development tutorial
- [The Embedded Rust Book](https://docs.rust-embedded.org/book/) - no_std Rust programming
- [Operating Systems: Three Easy Pieces](http://pages.cs.wisc.edu/~remzi/OSTEP/) - Operating system theory
- [OSDev Wiki](https://wiki.osdev.org/) - Comprehensive OS development reference

### Reference OS Projects
- [Redox OS](https://github.com/redox-os/redox) - Unix-like OS written in Rust
- [Theseus OS](https://github.com/theseus-os/Theseus) - Modular runtime system
- [Tock OS](https://github.com/tock/tock) - Ultra-low power OS for embedded systems
- [IntermezzOS](https://intermezzos.github.io/) - Minimal OS for learning

### Hardware References
- [Intel 64 and IA-32 Architectures Software Developer's Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [ACPI Specification](https://uefi.org/specifications)
- [UEFI Specification](https://uefi.org/specifications)

## ⚠️ Warnings

This project is **experimental** and under development.

- Do not use in production environments
- Do not test on systems with important data due to risk of data loss
- Use dedicated test machines when testing on actual hardware
- Kernel-level bugs can completely freeze the system

## 📧 Contact

For project-related inquiries, please contact us through [Issues](https://github.com/yourusername/simple_os_for_my_laptop/issues).

---

**Made with ❤️ and Rust**
