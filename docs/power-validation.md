# Power Validation Guide

## Overview

This guide describes how to measure and validate power consumption and stability metrics for Simple OS.

## Boot Profiles

- `power_saver`: Maximum power saving, screen blanks after 60s
- `balanced`: Balanced performance and power
- `performance`: Maximum performance
- `headless`: Headless mode (no GUI)

## Quick Validation Checks

### Shell Commands
- `power status` - Display current power statistics
- `power mode powersave` - Switch to power saving mode
- `power display off` - Turn off display
- `power disk idle 30000` - Set disk idle timeout to 30 seconds

### Idle Behavior
- GUI stops rendering when idle
- Display blanks after 60s (power_saver profile)
- CPU enters deep C-states
- Wakeup rate should be ≤ 50 wakeups/s

### Device Power Management
- **Disk**: Goes to standby after idle timeout; wakes on I/O
- **NIC**: RX stops after 10s idle; resumes on send/receive
- **Display**: Backlight dims/blanks based on policy

## Measurement Methods

### Boot Timeline

Capture boot timeline using `scripts/debug.sh`:
```bash
./scripts/debug.sh
```

The script creates a `boot_timeline_YYYYMMDD_HHMMSS.log` file with timestamps for each initialization phase.

### Idle Power Measurement

Use `scripts/power-test.sh` for 10-minute idle power measurement:
```bash
./scripts/power-test.sh
```

This generates:
- `power_idle_YYYYMMDD_HHMMSS.csv` - Power statistics in CSV format
- `power_test_YYYYMMDD_HHMMSS.log` - Full log output

### CSV Format

**power_idle.csv**:
```
timestamp,pkg_w,core_cstate_residency,wakeups_per_s
1000,2500,75.5,45.2
2000,2480,76.1,44.8
...
```

**suspend_cycles.csv** (when suspend/resume is implemented):
```
cycle_id,result,resume_ms,failures
1,success,120,0
2,success,115,0
...
```

## Target Metrics

### Phase 0 (Baseline)
- Boot timeline report generated
- Idle power CSV output
- C-state residency CSV output
- Crash dump with symbols

### Phase 1 (Idle Power Foundations)
- Idle residency ≥ 50%
- Idle power ≤ 3.0 W

### Phase 4 (Device Runtime PM)
- Idle ≤ 2.5 W
- Wakeup rate ≤ 50 wakeups/s

## QEMU Validation

- Verify CPU usage near 0% at idle
- Logging shows power state transitions
- Check C-state residency percentages
- Monitor wakeup rate

## Hardware Validation (HP 14s-dk0112AU)

- Check temperature reduction in power_saver mode
- Verify fan speed reduction
- Measure actual power consumption with external meter
- Compare with QEMU measurements

## Continuous Testing

### Nightly Tests
- Idle power test (10 minutes)
- Boot timeline validation

### Weekly Tests
- Suspend/resume marathon (50 cycles)
- Stress test (24 hours)

### Monthly Tests
- Filesystem corruption test (100 forced shutdowns)

