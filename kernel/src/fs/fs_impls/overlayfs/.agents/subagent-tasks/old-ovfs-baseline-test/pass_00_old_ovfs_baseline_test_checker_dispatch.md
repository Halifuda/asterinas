<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub

**Role ID:** CHECKER
**Pass Kind:** Meso Integration Pass (pre-design legacy baseline exception)
**Component/Task Group:** `old-ovfs-baseline-test`
**Parent Meso-Component:** `legacy_overlayfs_baseline_validation` (temporary validation parent; not an accepted Architect meso-component)
**Covered Micro-Features:**
- `P0-01`, `P0-02`, `P0-03`, `P0-04`, `P0-05`, `P0-08`, `P0-09`, `P0-10`, `P0-11`, `P0-12`, `P0-14`, `P0-15`, `P0-18`
- `P1-02`, `P1-03`, `P1-04`, `P1-06`, `P1-07`, `P1-08`, `P1-10`, `P1-12`, `P1-13`, `P1-16`, `P1-18`, `P1-21`, `P1-22`, `P1-23`, `P1-24`, `P1-25`, `P1-26`, `P1-27`, `P1-28`, `P1-29`, `P1-30`, `P1-31`, `P1-32`, `P1-34`
- `P2-01`, `P2-02`, `P2-06`, `P2-07`, `P2-11`, `P2-12`, `P2-13`, `P2-14`
- `P3-01`, `P3-02`, `P3-03`, `P3-04`, `P3-05`, `P3-08`, `P3-09`

This is a pre-design baseline exception authorized by the main agent. It
collects legacy behavior evidence only; it does not create a Designer
contract, a Creator pass, or official implementation acceptance.

## 1. Input Context (Read-Only)

- `kernel/src/fs/fs_impls/overlayfs/.agents/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/XFSTESTS_LIGHTWEIGHT_TRIAGE.md`
- `/home/ayd/asterinas/.agents/skills/ovfs-checker/SKILL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/main-agent/20260720-overlayfs-priors-complete_main_agent_handoff.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/qemu.log`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/qemu-serial.log`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/overlay-smoke.list`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/overlay-full.list`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/xfstests.config`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/common_rc_asterinas_compat.sh`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/MICRO_FEATURE_INVENTORY.md`
- `test/initramfs/src/conformance/xfstests/README.md`
- `test/initramfs/src/conformance/xfstests/run_xfstests.sh`
- `test/initramfs/src/conformance/xfstests/overlay/config/xfstests.config`
- `test/initramfs/src/conformance/xfstests/overlay/config/build_config.mk`
- `test/initramfs/src/conformance/xfstests/overlay/prepare.sh`
- `test/initramfs/src/conformance/xfstests/overlay/common_rc_asterinas_compat.sh`
- `test/initramfs/src/conformance/xfstests/overlay/run_list/short.list`
- `test/initramfs/src/conformance/xfstests/overlay/run_list/full.list`
- `test/initramfs/src/conformance/xfstests/overlay/run_list/block.list`

Do not read or modify the refactor design area or production implementation
unless a preserved runtime trace requires a narrowly scoped owner-boundary
lookup; this packet is evidence-first.

## 2. Output Requirement

- **Required Template:** `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/old-ovfs-baseline-test/evidence/pass_00_old_ovfs_baseline_test_checker.md`
- **Required supplemental evidence:** Write the controlled-rerun final table at `kernel/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv` and update it immediately after every rerun case completes or is killed. Historical provisional results are not imported. Do not retain per-case logs, per-case manifests, image copies, or full per-case commands.

The final report must use the controlled-rerun table for a complete case matrix for all 80 targets in
`overlay/run_list/full.list`. For this rerun, result classification has one
authoritative source: the `qemu.log` produced by that case's QEMU. Preserve
the xfstests result shown there: a passing result is `PASS`, an output or test
failure is `FAIL`, and an explicit `[not run]` result is `NOTRUN`. If that
QEMU has not produced a terminal case result by the 300-second wall-clock
cutoff, kill it and record `HANG/TIMEOUT`. Do not infer a result from
`qemu-serial.log`, guest result files, e2fsck, mount disappearance, image
health, controller state, or inability to map an unrelated log. Do not use
`CORRUPT` in this rerun. For every `FAIL` or `HANG/TIMEOUT`, include the
mandatory `Reproduce Command`, `Failed Test`, and `Evidence` fields using the
canonical command template and compact status row; do not copy full per-case
commands or logs into the workspace.

## 3. Specific Overrides & Commands

- Do not execute this packet until the main agent receives explicit user authorization.
- Use only the serialized `codex-asterinas-dev` `$ovfs-checker` lane. Confirm no competing QEMU job is live before each case.
- Do not run `short.list` or `full.list` as a multi-case batch. For the controlled rerun, start again at `overlay/001`, enumerate the 80 cases from `full.list`, write exactly one current case into the single reusable upstream-approved temporary runlist `test/initramfs/src/conformance/xfstests/overlay/run_list/baseline-single.list`, and launch one fresh QEMU for that case.
- A fresh QEMU is not sufficient isolation: before the controlled rerun and before every new case, after confirming the prior QEMU and its `make run_kernel` descendants have stopped, remove only the exact generated `test/initramfs/build/xfstests_test.img` and `test/initramfs/build/xfstests_scratch.img` files so the canonical `make run_kernel` path recreates and reformats a clean TEST/SCRATCH pair. Verify fresh mtime, size, and ext2 metadata before starting the case. Do not continue the matrix if image freshness cannot be proven.
- If those exact generated files are owned by `nobody:nogroup` and ordinary unlink fails, use the already authorized privileged `codex-asterinas-dev` container as root solely to remove those two exact image paths, then verify both are absent. Do not chmod or delete the broader build directory, and report an infrastructure block if the privileged cleanup is unavailable.
- Use a 300-second wall-clock cutoff per case. If the case has not completed by then, kill only that QEMU, immediately append `case<TAB>HANG/TIMEOUT` to the status table, and continue only with a fresh case after confirming the execution lane is clear.
- After every case, classify only the terminal result in that case's `qemu.log`. An xfstests `[not run]` line is `NOTRUN`; it must not be conflated with an orchestration or attribution problem. If the log is incomplete because the 300-second cutoff fired, record `HANG/TIMEOUT`.
- Do not classify image contamination, mount disappearance, post-QEMU health, controller races, or an unmappable log as `CORRUPT` or `NOTRUN`; stop and report a harness block if the case's own QEMU log cannot be preserved and identified.
- The prior matrices and reports have been cleared for this rerun. Create a fresh `kernel/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv` with one header and one row per target, and flush each row immediately after its QEMU exits or is killed. Do not import historical full-list results into it.
- For a terminal qemu.log result append exactly `case<TAB>PASS`, `case<TAB>FAIL`, or `case<TAB>NOTRUN`; append `case<TAB>HANG/TIMEOUT` only when the 300-second cutoff kills that case's QEMU. Never prefill unexecuted rows with `NOTRUN`.
- Transiently inspect only the current case's `qemu.log` for classification. Other logs and state may be used for lifecycle diagnostics, but never override the qemu.log result. Do not archive per-case logs or full commands. The status table is the durable execution record.
- Remove the exact packet-created `baseline-single.list` after all cases finish or the task stops. Do not modify `short.list`, `full.list`, `block.list`, the overlay config, or production logic, and do not leave additional single-case list files.
- Use this canonical command shape for every controlled-rerun case, changing only the reusable runlist content:

  ```bash
  docker exec -w /root/asterinas codex-asterinas-dev \
    make run_kernel AUTO_TEST=conformance RELEASE=1 MEM=12G \
    CONFORMANCE_TEST_SUITE=xfstests XFSTESTS_FS_TYPE=overlay \
    XFSTESTS_DISK_SIZE=8G \
    XFSTESTS_RUNLIST=/opt/xfstests/overlay/run_list/baseline-single.list
  ```

- Do not rerun the historical smoke list or re-investigate overlay xfstests startup. The previous smoke run and preserved handoff already establish the usable startup flow; begin directly with the isolated cases from `full.list`.
- Do not add or modify `#[ktest]`, `test_support/`, memory-disk fixtures, or any filesystem-local test code. Do not edit `SYSTEM_BLUEPRINT.md`, `PASS_SLICING.md`, or any main-agent handoff.
- This report is a baseline evidence receipt, not an acceptance of the future refactor and not a repair patch. If the run is blocked by infrastructure or missing result evidence, stop and report `HARNESS_OR_ENV` or `INCONCLUSIVE` rather than guessing.
