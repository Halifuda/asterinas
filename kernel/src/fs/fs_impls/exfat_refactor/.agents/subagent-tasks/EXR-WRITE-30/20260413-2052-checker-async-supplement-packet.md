<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-2052-CHECK-ASYNC-SUPPLEMENT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-2052-checker-async-supplement-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-WRITE-30`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 20:52 CST`

## Goal

- Check the landed `EXR-WRITE-30` async supplement in `inode.rs`: prove the new owner-private `publication_gate` satisfies the accepted write-side serialization contract without reopening resize ownership, and prove the earlier buffered-write slice still passes its exact-name regressions.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write and committed-growth publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`

## Required Resolution Questions

- Prove the earlier buffered-write/growth slice still passes after the new publication seam landed.
- Prove the new `publication_gate` keeps `write_at()`, `read_at()`, and `PageCacheBackend::npages()` on one owner-local published state.
- If checker needs one compact local `#[ktest]` in `inode.rs` to prove the publication seam, add it and keep it narrowly focused on buffered-write publication rather than resize/truncate semantics.
- If checker finds a strictly local `inode.rs` bug, make the narrowest in-scope fix and record it.
- Do not widen into resize/truncate/deallocation ownership, direct I/O, or sync ordering.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/14_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts
- reviewer and advisor artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- `EXR-WRITE-30` no longer owns resize/truncate publication.
  - `EXR-RESIZE-37` now owns that deferred control surface and its missing release/reclaim handshake.
- Treat the applicable `03_designer_ktest.md` obligations as buffered-write publication and committed-growth coverage only.
- `EXR-SYNC-31` remains the downstream owner of durable ordering and `write_page_async()`.
- `EXR-ALLOC-27` remains the committed-allocation owner, and `EXR-PGCACHE-26` remains the inode-local cache owner.

## Integration Prior Inputs

- The landed async supplement added one owner-private inode-local `publication_gate: RwLock<()>`.
- The landed code routes:
  - `write_at()` through the write side of that gate,
  - `read_at()` through the read side,
  - `PageCacheBackend::npages()` through the read side.
- The checker must prove that this publication seam does not regress the earlier buffered-write slice and does not pull resize/truncate back into `EXR-WRITE-30`.
- If one compact new ktest is needed, prefer a publication-focused name such as `inode_publication_gate_keeps_read_and_npages_on_one_published_state`.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Record whether execution used KVM or fell back to TCG.
- Exact test names are required; do not use fragments.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer report-only checking unless a strictly local `inode.rs` fix or compact local ktest is needed.
- Reject any change that invents a background writer, filesystem-global mutation coordinator, or second publication seam.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_exercises_buffered_read'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_grows_an_empty_file_and_publishes_allocation_facts'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_buffered_write_extends_a_non_empty_file_across_growth'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_publication_gate_keeps_read_and_npages_on_one_published_state'`
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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/17_checker_serial.md`

## Escalation Rule

- If the checker needs edits outside `inode.rs`, or if proving the new publication seam would require reopening resize/truncate ownership instead of a bounded buffered-write check, report that and stop.
