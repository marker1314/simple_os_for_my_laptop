# Power Validation Guide

- Profiles: power_saver, balanced, performance, headless
- Quick checks:
  - Shell: `power status`, `power mode powersave`, `power display off`, `power disk idle 30000`
  - Idle: GUI stops rendering when idle; display blanks after 60s (power_saver)
  - Disk: goes to standby after idle timeout; wakes on I/O
  - NIC: RX stops after 10s idle; resumes on send/receive

- QEMU: verify CPU usage near 0% at idle; logging shows transitions.
- Hardware (HP 14s-dk0112AU): check temps/fan reduction in power_saver.

