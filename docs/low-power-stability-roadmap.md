## Low Power and Stability Roadmap

### Scope and goals
- Focus: laptop usage under Simple OS with emphasis on idle power, sleep/wake reliability, and crash-free uptime.
- Time horizon: 6–10 weeks across phased, shippable increments.
- Out of scope: new UI features; non-critical peripherals beyond keyboard/trackpad/display/network/storage.

### Key measurable targets (acceptance before public preview)
- Idle platform power at desktop: ≤ 2.5 W on reference laptop (iGPU), screen on 30%.
- s2idle cycles: ≥ 50 consecutive suspend/resume cycles, success ≥ 98%.
- Deep idle residency: ≥ 70% package C-state residency during 10 min idle.
- Crash-free session: ≥ 24 h with stress-ng mix, no kernel panic, no memory leak growth > 2%.
- Filesystem robustness: 100 unclean shutdowns with zero metadata corruption and ≤ 1% fsck repairs.

### Phase 0 — Baseline and observability (Week 0–1)
- Implement/verify structured logging and ring buffer exposure (`src/logging.rs`).
- Crash path verification: ensure `src/crash.rs` produces symbolized dumps and reboot-safe markers.
- Power counters: wire `src/power/stats.rs` to sample C/P states, CPU residency, and RAPL/EC if available.
- CLI tooling: extend `scripts/debug.sh` and `scripts/run.sh` to capture boot timings and power stats.
- Doc: align with `docs/power-validation.md`; create runbooks and a simple dashboard CSV schema.

Deliverables:
- Boot timeline report, idle power CSV, residency CSV, crash dump with symbols.

### Phase 1 — Idle power foundations (Week 1–2)
- CPU idle: refine `src/power/idle.rs` to use HLT/MWAIT and honor `src/power/policy.rs` thresholds.
- Scheduler cooperation: ensure `src/scheduler/load_balancer.rs` consolidates load to allow deeper C-states.
- Timer tick policy: coalesce timers in `drivers/timer.rs` to reduce periodic wakeups (target ≥ 10 ms idle tick).
- Framebuffer vblank: avoid busy loops in `drivers/framebuffer.rs`; move to event-driven redraw in `src/gui/compositor.rs`.

Exit criteria:
- Idle residency ≥ 50%, idle power ≤ 3.0 W.

### Phase 2 — Suspend/Resume stability (Week 2–3)
- ACPI flows: complete S3/s2idle in `src/power/acpi.rs`, quiesce/resume order via `src/power/manager.rs` hooks.
- Device quiesce: implement `power_off/power_on/save/restore` in `src/power/device.rs` for storage, NIC, input, display.
- Interrupt mask/unmask: verify `src/interrupts/pic.rs` and `src/interrupts/idt.rs` consistency across resume.
- State tests: 50× suspend/resume script with randomized network and I/O load.

Exit criteria:
- ≥ 98% successful cycles; no lost interrupts; no device wedged states.

### Phase 3 — CPU frequency and energy policy (Week 3–4)
- Scaling: implement governors in `src/power/scaling.rs` (ondemand/powersave/performance) with hysteresis.
- RAPL (if Intel): stabilize `src/power/rapl.rs` sampling and budget enforcement where supported.
- Thermal: wire `src/power/temps.rs` to throttle policy hooks; protect from thermal runaway.

Exit criteria:
- Idle ≤ 2.8 W with powersave, interactive latency p95 ≤ 50 ms on window focus/type.

### Phase 4 — Device runtime power management (Week 4–5)
- PCIe ASPM/clock gating where feasible via `src/drivers/pci.rs`.
- Network: `src/net/driver.rs` implement low-power idle; `drivers/rtl8139.rs` ensure no periodic tx/rx polls.
- Input: `src/drivers/i2c_hid.rs` and `src/drivers/touchpad.rs` switch to interrupt-driven reporting.
- Display: backlight control via policy; avoid full repaint when occluded/minimized (`src/gui/compositor.rs`).

Exit criteria:
- Idle ≤ 2.5 W, wakeup rate ≤ 50 wakeups/s at desktop.

### Phase 5 — Filesystem robustness and write amplification (Week 5–6)
- Journaled writes: add journal or soft-updates to `src/fs/fat32.rs` or introduce a minimal log layer in `src/fs`.
- Cache writeback policy: tune `src/fs/cache.rs` for bursty flush; reduce sync writes on metadata paths.
- fsck tooling: extend `src/fs` with offline repair hooks; add corruption-injection tests.

Exit criteria:
- 100 forced power-offs: zero data loss on closed files; no mount failures.

### Phase 6 — Kernel stability and memory safety (Week 6–7)
- Guard pages for stacks in `src/memory/paging.rs`; enable redzones in `src/memory/slab.rs`.
- Double-free/UF detection toggles in debug builds; periodic allocator consistency check.
- Watchdog: soft lockup detector tied to scheduler tick; panic path proves useful dump.

Exit criteria:
- 24 h stress run: no panic; memory footprint drift ≤ 2%.

### Cross-cutting validation
- Automated suites: expand `tests/README.md` guidance with scripts for power/stability scenarios.
- Metrics schema (CSV):
  - power_idle.csv: timestamp, pkg_w, core_cstate_residency, wakeups_per_s
  - suspend_cycles.csv: cycle_id, result, resume_ms, failures
  - fs_robustness.csv: run_id, unclean_count, repairs, data_loss
- Continuous runs: nightly idle, weekly suspend marathon, monthly corruption tests.

### Risk register and mitigations
- ACPI variance: lock to a reference laptop SKU; maintain HCL and quirk table in `src/power/policy.rs`.
- GPU/display variability: begin with simple framebuffer; document iGPU model constraints.
- NIC power idle: allow fallback to wired first; defer complex Wi‑Fi PM if unstable.

### Release criteria (public preview)
- Meets all targets under “Key measurable targets”.
- HCL lists the reference laptop; known issues documented in `docs/README.md` and `roadmap.md`.
- Rollback-safe updates: if an update fails, previous kernel/filesystem state reboots cleanly.

### Pointers to code areas (for implementers)
- Power core: `src/power/{manager.rs,policy.rs,stats.rs,idle.rs,scaling.rs,rapl.rs,temps.rs}`
- Devices: `src/drivers/{pci.rs,timer.rs,framebuffer.rs,i2c_hid.rs,touchpad.rs,rtl8139.rs}`
- Kernel: `src/scheduler/*`, `src/interrupts/*`, `src/memory/*`, `src/crash.rs`, `src/logging.rs`
- Filesystem: `src/fs/{fat32.rs,cache.rs,vfs.rs}`

### How to run the validation quickly
- Idle: boot to desktop, wait 2 min, capture stats for 10 min; verify residency/power.
- Suspend: run suspend loop 50× with background I/O and network traffic; record success and resume time.
- FS: write 1 GB dataset, force power-offs at random intervals, check mount and integrity after 100 cycles.

> See also: `docs/power-validation.md` for measurement methods and tooling.

