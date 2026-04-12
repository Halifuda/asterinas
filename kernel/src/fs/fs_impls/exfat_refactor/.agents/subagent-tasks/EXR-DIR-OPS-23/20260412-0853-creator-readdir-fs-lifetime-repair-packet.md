<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260412-0853-CREATOR-READDIR-FS-LIFETIME-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260412-0853-creator-readdir-fs-lifetime-repair-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-DIR-OPS-23`
- Phase: `serial repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 08:53 CST`

## Goal

- Repair the currently known test-owned `readdir_*` lifetime bug so the tests keep the owning `Arc<ExfatFs>` alive while calling `root.readdir_at(...)`.
- Keep the repair inside `inode.rs` test code only.

## Required Inputs

- Prior continuity:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/harbor-lattice-20260412-0906-pruned-readdir-misdiagnosis-and-review.md`
- Relevant code:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Read-only references:
  - `/home/halifuda/asterinas/qemu-serial.log`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/harbor-lattice-20260412-0906-pruned-readdir-misdiagnosis-and-review.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/qemu-serial.log`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/22_creator_serial_readdir_fs_lifetime_repair.md`

## Forbidden Scope

- No edits to production `lookup`, production `readdir_at`, `fs.rs`, or `directory.rs`.
- No packet, board, handoff, or protocol edits.
- No test-behavior widening beyond what is required to keep the owning filesystem alive.

## Role References To Read

- `$exfat-subagent-workflow`
- `references/creator.md`

## This-Round Change Requirements

- Treat the current known failure as test-owned:
  - `prepared_clean_directory_root()` returns `(Arc<ExfatFs>, Arc<ExfatInode>)`
  - the two `readdir_*` tests currently bind it as `let (_, root) = ...`
  - `root.readdir_at(...)` then reaches `ExfatInode::owner_fs()` after the owning `Arc<ExfatFs>` has been dropped
- Repair only the two `readdir_*` tests so they keep the returned filesystem owner alive for the duration of the test.
- Prefer the narrowest repair shape, such as binding `_fs` or another intentionally-kept local strong reference.
- Do not reopen the fixture shape, visitor semantics, production `readdir_at`, or `.` / `..` behavior in this packet.

## Landing Discipline

- Test-only change set inside the existing `#[cfg(ktest)]` block.
- No new production helpers, no new owner-facing seams, and no changes to the current component boundary.

## Evidence To Record

- State exactly which tests were repaired and how the strong `Arc<ExfatFs>` lifetime is preserved.
- State explicitly that production code remained untouched.
- If you discover that the lifetime repair alone forces additional test edits, stop and report the exact blocker instead of widening scope.

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/22_creator_serial_readdir_fs_lifetime_repair.md`
