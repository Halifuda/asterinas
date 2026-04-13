<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-1821-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1821-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-WRITE-30`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 18:21 CST`

## Goal

- Check the landed `EXR-WRITE-30` buffered-write slice after the creator repair: prove the current `write_at` / growth path executes correctly with exact-name ktests, classify any strictly local issue in `inode.rs`, and record that `resize` remains an intentionally deferred follow-on slice rather than pretending the full row is already closed.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write, committed growth, and visible-byte publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`

## Required Resolution Questions

- Prove the landed buffered-write slice now executes successfully for:
  - gap zero-fill within existing allocation,
  - empty-file growth through committed allocation,
  - non-empty-file growth that stitches newly allocated clusters onto the preexisting chain.
- If checker finds a strictly local `inode.rs` bug or missing small test adjustment, make the narrowest in-scope fix and record it.
- Record clearly that `resize` is still deferred and therefore remains unverified in this pass; that is not by itself a checker failure for this packet.
- If execution fails for reasons outside `inode.rs` or the checker artifact, classify that and stop.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts
- reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- This checker pass validates the currently landed buffered-write slice only.
- `resize`, truncate, direct I/O, and sync ordering remain outside this checker packet and outside the current landed slice.
- `EXR-ALLOC-27` remains the only owner of committed allocation; the write owner may only consume its results.
- Exact `cargo osdk test` names are required. Do not use broad prefixes or fragments.

## Integration Prior Inputs

- The exact landed write-slice regression names are:
  - `inode_carrier_snapshots_metadata_and_exercises_buffered_read`
  - `inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts`
  - `inode_buffered_write_extends_a_non_empty_file_across_growth`
- Use those complete names directly in `cargo osdk test ...` commands.
- If checker determines one more small local `inode.rs` ktest is required to prove the landed slice, it may add that test, but only if the addition stays compact and within the packet boundary.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Record whether execution used KVM or fell back to TCG.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying it.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer report-only checking unless a strictly local `inode.rs` fix is necessary.
- Do not turn the checker into a resize or redesign pass.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_extends_a_non_empty_file_across_growth'`
- If the guest-side failure is unclear after a nonzero run, read-only inspection of `/home/halifuda/asterinas/qemu-serial.log` is allowed.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Execution Environment

- Host and Docker
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-WRITE-30 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`

## Escalation Rule

- If the checker needs edits outside `inode.rs` or cannot get trustworthy exact-name test evidence from the allowed commands, report that and stop.
