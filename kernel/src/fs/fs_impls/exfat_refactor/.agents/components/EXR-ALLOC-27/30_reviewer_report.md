<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` cluster allocation service boundary
- Status: `Reviewed`
- Author: reviewer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-2231-reviewer-packet.md`
- Reviewed implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Review Scope

- Reviewed the landed allocator service and local helper surfaces across `allocator.rs`, `bitmap.rs`, `fat.rs`, and `fs.rs`.
- Reviewed the checker-owned evidence in `11_checker_serial.md` as the authoritative runtime proof for this review pass.
- Focused on owner-boundary discipline, committed-result shape, commit visibility, and whether the sector-aligned metadata-write repair stayed subordinate to allocator ownership.

## Findings

No findings.

## Review Notes

- `Allocator` remains owner-private to `ExfatFs` and does not widen into a standalone free-space manager or reservation lease API.
- `AllocationResult` stays intentionally small and copyable, so later namespace and write rows can consume committed allocation facts without pulling bitmap or FAT internals across the owner boundary.
- The sector-aligned read-modify-write helpers in `bitmap.rs` and `fat.rs` remain subordinate to the allocator commit path; they repair the block-device contract locally instead of introducing a new storage service boundary.
- The checked implementation still publishes the in-memory bitmap snapshot only after the on-disk commit succeeds, which matches the designed “private until committed” visibility rule.

## Production Changes

- No production code changed in this review pass.
- This was a report-only review artifact.

## Outcome

- The reviewed implementation stays within the accepted designer boundary.
- The checker-owned exact-name proofs plus this clean review are sufficient for main-agent acceptance without another final-check rerun.
