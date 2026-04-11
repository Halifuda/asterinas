<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-21-CHECKER-20260410-1335`
- Date: `2026-04-10`
- Scheduler: `main-agent`
- Skill: `$exfat-subagent-workflow`
- Role: `checker`
- Component: `EXR-BITMAP-21`
- Phase: `Serial checker`

## Goal

Validate the new `ExfatFs` allocation-bitmap owner boundary in `bitmap.rs` and `fs.rs`.
Own the minimal local `#[ktest]` coverage needed to prove validation-before-publication, occupancy queries, and derived accounting from one immutable snapshot.
If the first run fails or hangs, use bounded checker-only debug output and/or a debug-oriented `cargo osdk test` rerun inside the allowed write set to localize the issue, then remove temporary debug edits before finishing.

## Required Inputs

- Component architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- Component designer artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/03_designer_ktest.md`
- Creator artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- Production files to inspect:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Shared testing references:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/11_checker_serial.md`

## Forbidden Scope

- No edits outside the write set.
- No production widening into allocation search, allocator mutation policy, FAT writes, directory traversal, mount/open sequencing, or namespace operations.
- No board edits, handoff edits, packet rewrites, or protocol edits.
- Do not leave temporary debug output or checker-only comments in production files after the final recorded pass.

## Role References To Read

- `$exfat-subagent-workflow`
- `references/common-subagent.md`
- `references/checker.md`
- `references/testing-guide.md`

## This-Round Semantic Constraints

- Preserve `ExfatFs` as the only owner; `AllocationBitmap` stays an owner-local immutable snapshot, not a new allocator or search service.
- Validation must still happen before publication.
- `cluster_is_allocated()`, `used_cluster_count()`, and `free_cluster_count()` must all derive from the same published snapshot.
- Cluster `2` must still map to bitmap bit `0`.
- Padding bits outside the valid cluster range must not affect accounting.

## This-Round Integration Constraints

- Prefer placing new bitmap regressions in `bitmap.rs` if possible; use `fs.rs` only if the owner-visible API makes that materially clearer.
- Keep tests small and local.
- You may add temporary checker-only debug output inside `bitmap.rs` or `fs.rs` if the first run fails or hangs, but remove it before the final pass you record.
- You may use a debug-oriented `cargo osdk test` rerun, including `--profile dev`, if needed to diagnose a failure or hang.

## Execution Environment

- Preferred container: `codex-asterinas-dev`
- Repository path in container: `/root/asterinas`
- Preferred command form:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-or-justified-suffix>'`
- Lock acquire command:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire`
- Lock release command:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`

## Allowed Commands

- `sed`, `rg`, `git diff`, `git status`, `cargo fmt --all`
- `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire`
- `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <filter>'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test --profile dev <filter>'`

## Required Test Coverage

- One regression that proves invalid bitmap images are rejected before publication.
- One regression that proves occupancy queries match the published bitmap bytes for first, middle, and tail data clusters.
- One regression that proves used/free accounting matches the same snapshot and ignores padding bits beyond the valid range.

## Evidence Requirements

- Record the exact filtered test commands.
- Record whether `/dev/kvm` appeared visible before the run and whether the actual run looked like KVM or TCG.
- Prove that each filter hit the intended test, either by source-backed exact suffixes or by command output listing the executed tests.
- Classify any failure as environment, build, or test failure.
- If you use temporary debug output or debug-profile reruns, record why and confirm the final recorded state removed the temporary debug edits.

## Stop Condition

Stop after one checker artifact:

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/11_checker_serial.md`

The artifact must state one of:

- pass with executable evidence,
- build failure with exact blocker,
- environment failure with exact blocker,
- test failure with exact failing scenario.
