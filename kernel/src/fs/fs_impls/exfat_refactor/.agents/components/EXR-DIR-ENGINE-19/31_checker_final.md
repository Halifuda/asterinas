<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `DirectoryEngine` Read-Only Record Stream
- Status: `FinalChecked`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1200-checker-final-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Pass kind: `post-review final`

## Scope of Review

Reviewed the reviewer hardening in `directory.rs` against the designer boundary and the prior checker/reviewer artifacts.

The review checked that unexpected top-level dentries now fail instead of surfacing as generic singleton candidates, that the engine remains read-only and owner-internal, and that the local regression coverage proves the hardening through the filtered `directory_engine_` test set.

## Test Changes

- Added one local `#[ktest]` regression in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`.
- The regression is `directory_engine_rejects_unexpected_top_level_dentry()`.
- The existing checker-owned tests remain in the same file and still have short scenario comments.

## Findings

No findings.

## Verified Properties

- The reviewer hardening is present at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:104-108`, where unexpected top-level dentries now return `EINVAL`.
- The new regression at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:441` exercises that behavior directly.
- The filtered command `cargo osdk test directory_engine_` completed successfully under the checker lock.
- `/dev/kvm` was present, but the run emitted TCG warnings, so the final verification used TCG-backed QEMU rather than KVM acceleration.
- Source-backed suffix proof confirms the filter covered all five `directory_engine_` tests in `directory.rs`, including the new hardening regression.

## Unverified Properties

- None.

## Recommendation

- Next owner: `main-agent`
- Reason: Final checker verification is complete and the reviewer hardening is backed by a passing filtered run.
- Blocking or non-blocking: Non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: Required final checker pass.

