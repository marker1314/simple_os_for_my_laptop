# Power and Stability Runbooks

## Overview

This document provides operational procedures for measuring and validating power consumption and stability metrics in Simple OS.

## Boot Timeline Capture

### Purpose
Capture detailed timing information for each kernel initialization phase to identify boot bottlenecks.

### Procedure
1. Run `scripts/debug.sh`:
   ```bash
   ./scripts/debug.sh
   ```
2. The script generates `boot_timeline_YYYYMMDD_HHMMSS.log`
3. Analyze the log for timing information:
   - Serial port initialization
   - Boot info initialization
   - PIC remapping
   - IDT initialization
   - Memory management initialization
   - Timer driver initialization
   - Power management initialization
   - Driver initialization

### Expected Output
```
[INFO] Simple OS Kernel Starting...
[INFO] Boot info initialized
[INFO] PIC remapped
[INFO] IDT initialized
[INFO] Memory management initialized successfully
[INFO] Timer driver initialized
[INFO] Power management initialized
```

## Power Statistics Collection

### Purpose
Measure idle power consumption and C-state/P-state residency.

### Procedure
1. Boot the system to desktop
2. Wait 2 minutes for stabilization
3. Run `scripts/power-test.sh` for 10-minute measurement:
   ```bash
   ./scripts/power-test.sh
   ```
4. Or manually collect stats:
   - Boot system
   - Wait for idle state
   - Export CSV using kernel functions (via shell or syscall)

### CSV Export Format

**power_idle.csv** columns:
- `timestamp`: Elapsed time in milliseconds since boot
- `pkg_w`: Package power in milliwatts
- `core_cstate_residency`: C-state residency percentage
- `wakeups_per_s`: Wakeup rate (wakeups per second)

**Example**:
```csv
timestamp,pkg_w,core_cstate_residency,wakeups_per_s
10000,2500,75.5,45.2
20000,2480,76.1,44.8
30000,2470,77.2,43.5
```

## Crash Dump Analysis

### Purpose
Analyze kernel crashes and exceptions with stack traces.

### Procedure
1. Boot the system after a crash
2. Check for previous crash dump:
   ```
   [WARN] Previous crash detected
   === Crash Dump ===
   Reason: EXCEPTION (code: 0xd)
   RIP: 0x0000000000101234
   RBP: 0x0000000000205678
   Stack Trace (3 frames):
     #0: 0x0000000000101234
     #1: 0x0000000000101567
     #2: 0x0000000000101890
   ==================
   ```
3. Use RIP addresses with debug symbols:
   ```bash
   rust-objdump -d target/x86_64-unknown-none/debug/simple_os | grep <RIP>
   ```

### Symbolization
- RIP addresses are in kernel virtual address space
- Use debug symbols from `target/x86_64-unknown-none/debug/simple_os`
- Stack trace shows call chain up to 8 frames

## Log Export

### Purpose
Export structured logs for analysis.

### Procedure
1. Boot the system
2. Trigger log export (via shell command or syscall)
3. Export formats:
   - **Recent logs**: `logging::dump_recent()`
   - **Filtered by level**: `logging::dump_by_level(LogLevel::Error)`
   - **CSV format**: `logging::export_csv()`

### CSV Log Format
```csv
timestamp_ms,level,message
1000,INFO,"Boot info initialized"
2000,INFO,"PIC remapped"
3000,ERROR,"Page fault at 0x12345678"
```

## Power Test Dashboard

### Simple Dashboard Script

Create a simple Python script to visualize power metrics:

```python
import csv
import matplotlib.pyplot as plt

# Read power_idle.csv
timestamps = []
power_mw = []
cstate_residency = []
wakeups = []

with open('power_idle.csv', 'r') as f:
    reader = csv.DictReader(f)
    for row in reader:
        timestamps.append(float(row['timestamp']))
        power_mw.append(float(row['pkg_w']))
        cstate_residency.append(float(row['core_cstate_residency']))
        wakeups.append(float(row['wakeups_per_s']))

# Plot
plt.figure(figsize=(12, 8))

plt.subplot(2, 2, 1)
plt.plot(timestamps, power_mw)
plt.title('Package Power (mW)')
plt.xlabel('Time (ms)')
plt.ylabel('Power (mW)')

plt.subplot(2, 2, 2)
plt.plot(timestamps, cstate_residency)
plt.title('C-State Residency (%)')
plt.xlabel('Time (ms)')
plt.ylabel('Residency (%)')

plt.subplot(2, 2, 3)
plt.plot(timestamps, wakeups)
plt.title('Wakeup Rate (wakeups/s)')
plt.xlabel('Time (ms)')
plt.ylabel('Wakeups/s')

plt.tight_layout()
plt.savefig('power_dashboard.png')
```

## Troubleshooting

### High Idle Power
1. Check C-state residency: Should be > 50%
2. Check wakeup rate: Should be < 50 wakeups/s
3. Verify device power management:
   - Disk in standby?
   - NIC in low-power mode?
   - Display blanked?

### Boot Timeout
1. Check boot timeline log
2. Identify slow initialization phase
3. Verify hardware initialization order

### Crash Analysis
1. Check crash dump for RIP and stack trace
2. Verify symbolization with debug symbols
3. Check related logs for context
4. Review code at RIP address

## Validation Checklist

### Phase 0 (Baseline)
- [ ] Boot timeline report generated
- [ ] Idle power CSV exported
- [ ] C-state residency CSV exported
- [ ] Crash dump with symbols displayed

### Phase 1 (Idle Power)
- [ ] Idle residency ≥ 50%
- [ ] Idle power ≤ 3.0 W
- [ ] C-state transitions logged

### Phase 4 (Device PM)
- [ ] Idle ≤ 2.5 W
- [ ] Wakeup rate ≤ 50 wakeups/s
- [ ] All devices in low-power mode

