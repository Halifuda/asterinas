<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Log

## Metadata

- Component ID: EXR-DENTRY-04A
- Title: Raw Dentry Layout And Typed Single-Entry Decode
- Status: `SerialChecked`
- Author: checker
- Date: 2026-04-01

## Summary

Validated the creator pass with checker-owned ktests covering raw entry size, special typed entry recognition, and deleted/unused plus generic fallback classification.

## Verification

- Preflight: `/dev/kvm` was not present in the container, so QEMU ran with TCG fallback.
- Command: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test raw_dentry_has_expected_size'`
- Result: passed under TCG.
- Command: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test typed_decode_recognizes_special_entry_kinds'`
- Result: passed under TCG.
- Command: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test typed_decode_handles_deleted_unused_and_generic_fallbacks'`
- Result: passed under TCG.

## Notes

- The first compile attempt failed because the new `#[cfg(ktest)]` module needed `use ostd::prelude::ktest;`. That was fixed in the test-only section of `dentry.rs` and then the same filtered test passed.
- I did not modify `COMPONENT_INDEX.md` or any main-agent artifacts.
- No reviewer or task-board updates were made.
