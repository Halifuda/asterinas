<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlayfs Workspace Gap Registry

Durable registry of kernel/workspace-level gaps discovered by this
refactor that are **outside the overlayfs implementation's own write-set**.
Each entry records the discovery evidence and the pending ownership
decision; closure requires a fix landing in the owning component.

## GAP-KTEST-001 — ktest output silently discarded (early-console gating)

- **Status**: OPEN — ownership decision pending (user).
- **Discovered**: 2026-08-31, unit-test Checker validation run
  (`task_checker_unit_tests_ktest_20260831`, run_1); documented in live
  handoff §12.
- **Symptom**: every aster-core `cargo osdk test` run produces a
  ~403-byte serial log containing only the OVMF/GRUB boot stub — zero
  kernel or ktest-runner output. Per-test assert messages and panic
  locations are invisible in the CI `make ktest` lane as well as locally;
  only the process exit code (0/1) is observable. Diagnosing a failing
  ktest requires per-test isolated runs (one QEMU boot each) or
  out-of-band measures.
- **Root cause**: `kernel/core/comps/cmdline/src/early.rs` registers a
  strong `#[ostd::early_cmdline_parser]` that overrides ostd's weak
  default (which enabled the early console unconditionally). The strong
  parser enables `has_early_console` only when the guest cmdline contains
  `earlycon`. The OSDK test kernel's cmdline is fixed to `["--"]` (no
  `earlycon`), so `SERIAL_PORT` is never initialized and every
  `early_print!` from the ktest runner is dropped. Introduced by parent
  commit `61e1ad700` (component migration); affects all aster-core ktest
  observability since then, including CI.
- **Workaround verified** (no repo change):
  `cargo osdk test --kcmd-args="earlycon"` — restores full runner output
  (403 → 4322 bytes evidenced in `qemu-serial.log`, run_27).
- **Ownership options** (decision pending):
  1. cmdline-crate side: make the strong early parser default
     `has_early_console` to true when no explicit console selection is
     present (restores pre-migration ostd default behavior).
  2. OSDK side: have `cargo osdk test` inject `earlycon` into the test
     kernel cmdline by default (scoped to the ktest lane, no production
     cmdline change).
  - Main-agent recommendation: option 1 (restores the pre-`61e1ad700`
    behavior for all lanes and keeps OSDK generic); option 2 is the
    smaller blast radius if the strong-parser behavior is otherwise
    intended.
- **Closure condition**: a full `make ktest`-equivalent run shows
  per-test output in the serial log without workarounds.
