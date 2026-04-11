<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `DirectoryEngine` read-only record stream
- Status: `SerialChecked`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1112-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Pass kind: `serial`

## Scope of Review

Checked the landed `DirectoryEngine` service against the architect and designer clauses for read-only record streaming, boundary-preserving record emission, singleton `Bitmap`/`Upcase` surfacing, tombstone skipping, and `Unused` termination.
I also reviewed the checker-owned local test coverage added in `directory.rs` and ran the filtered checker command under the execution lock.

## Test Changes

- Added four local `#[ktest]` scenarios in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`.
- The scenarios cover cluster-boundary record ordering, validated file-record emission, raw singleton surfacing, and deleted/unused handling.
- Each checker-owned test has a short scenario comment.

## Findings

### Finding

- Severity: 2
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs:230-256`
- Description: The filtered checker run does not complete because the crate still has an unrelated borrow-of-moved-value compile error in `fs.rs`, so no directory-engine runtime evidence can be collected from this pass.
- Violated spec clause or expected behavior: The checker packet required filtered executable verification and proof that the intended tests ran; the build failure blocks that evidence.
- Reproduction or reasoning: `cargo osdk test directory_engine_` compiles the crate far enough to reach `fs.rs`, then fails with `E0382` before any `directory_engine_` tests can execute.

## Verified Properties

- The checker lock was acquired before running `cargo osdk test`.
- `/dev/kvm` was present, so the filtered run used KVM-backed QEMU rather than TCG.
- The local checker-only test scaffolding in `directory.rs` compiled far enough for the build to progress past the directory module.
- The final observed failure was outside the checker write set and not caused by the directory-engine checker edits.

## Unverified Properties

- The four `directory_engine_` scenarios could not be executed because the unrelated `fs.rs` build failure stopped the filtered run first.
- I could not record runtime evidence for ordering, validated file-record emission, singleton surfacing, or tombstone termination from this pass.

## Recommendation

- Next owner: `EXR-FS-CORE-16` / the `fs.rs` owner
- Reason: the remaining blocker is an unrelated build failure in `fs.rs`, which must be cleared before the directory-engine checker pass can produce executable evidence.
- Blocking or non-blocking: Blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: Required serial checker pass; not a skip case.
