<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Scope

- Component: `EXR-DENTRY-WRITE-28`
- Role: `checker`
- Packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0700-checker-serial-packet.md`
- Focus: `DirectoryEngine` write-side mutation boundary in `directory.rs`

## Local Edits

- Added three local `#[ktest]` regressions in `directory.rs` for:
  - tombstoned slot reuse before growth,
  - in-place rewrite preserving the trusted location, and
  - committed-growth append behavior.
- Fixed the local checker test harness in `directory.rs` by importing `VmIo` for the write path and `Vec` for the new helper signatures.

## Verification

- KVM was present in the container: `/dev/kvm` existed, but QEMU reported TCG warnings in the guest run, so the actual ktest execution used TCG.
- Exact filtered evidence:
  - `cargo osdk test directory_engine_reuses_deleted_slots_before_growth`
  - `cargo osdk test directory_engine_preserves_location_when_rewrite_still_fits`
  - `cargo osdk test directory_engine_consumes_committed_growth_for_directory_expansion`
- Each exact run passed in `qemu-serial.log` with `1 passed; 151 filtered out`.

## Notes

- A broad `directory_engine_` filter did not prove any hits, so exact test names were used for final proof.
- One early compile failure was strictly local to `directory.rs` and was corrected in place before rerunning the same exact evidence.

## Result

- Checker pass completed successfully.
