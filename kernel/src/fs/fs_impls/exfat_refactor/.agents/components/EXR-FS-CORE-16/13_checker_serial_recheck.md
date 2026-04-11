<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-FS-CORE-16`
- Title: `ExfatFs` Filesystem Owner Boundary
- Status: `SerialChecked`
- Author: `main-agent (local checker loop using $exfat-main-agent and checker-role guidance from $exfat-subagent-workflow)`
- Date: `2026-04-10`
- Task packet: `local main-agent checker loop authorized by the user on 2026-04-10; no archived delegated packet`
- Checked implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Pass kind: `serial`

## Scope of Review

This recheck revalidated the three `EXR-FS-CORE-16` filtered ktests after the shared checker environment drifted away from the assumptions recorded in the 2026-04-07 retry report.

The loop first restored the missing initramfs build artifacts in the shared Docker container with `make initramfs`, then restored the repository working-tree executable bit on `/home/halifuda/asterinas/tools/qemu_args.sh`. That executable-bit drift mattered because `OSDK.toml` shells out through `$(./tools/qemu_args.sh ...)`; without it, the launcher dropped the intended QEMU serial and console arguments and the checker observed only partial boot output before opaque exits.

No production edits were required in `fs.rs` or `mod.rs`.

## Test Changes

No tests were added or modified in this recheck.

The previously added local `#[ktest]` coverage remains unchanged in `fs.rs`:

- `filesystem_identity_and_super_block_snapshot_are_stable`
- `subscriber_stats_and_snapshot_survive_placeholder_sync`
- `root_inode_temporary_seam_stays_on_file_system_owner`

The source-backed exact suffix proof from the earlier checker reports still applies because the local `#[ktest]` names are unchanged.

## Findings

No in-scope production-code defects were found in `fs.rs` or `mod.rs`.

## Verified Properties

- The shared checker lock was acquired before command-producing work and released afterward.
- The checker environment had drifted in two concrete ways before the rerun:
  - `/root/asterinas/test/initramfs/build/` was absent in the container until `make initramfs` rebuilt it.
  - `/home/halifuda/asterinas/tools/qemu_args.sh` had lost its executable bit in the working tree even though the repository tracks it as executable.
- After those environment repairs, all three filtered `EXR-FS-CORE-16` ktests exited successfully:
  - `filesystem_identity_and_super_block_snapshot_are_stable`
  - `subscriber_stats_and_snapshot_survive_placeholder_sync`
  - `root_inode_temporary_seam_stays_on_file_system_owner`
- `/dev/kvm` was visible in `codex-asterinas-dev`, but the observed QEMU output still printed TCG CPU-feature warnings, so the practical runtime mode remained TCG fallback for this pass.
- The opaque guest-tail problem reported by the user matched the missing-QEMU-args state: once the executable bit was restored, the checker again received the expected forwarded-port banner and boot-console output instead of a near-silent hang.

## Unverified Properties

- No additional output-backed proof naming the individual ktests was surfaced by the runner; this pass therefore still relies on exact source-backed suffix proof rather than explicit test-name echoes from guest output.
- This pass did not perform reviewer or post-review final-checker work.

## Recommendation

- Next owner: `main-agent`
- Reason: `EXR-FS-CORE-16` remains `SerialChecked`; reviewer work may proceed once the sibling `EXR-INODE-CORE-17` checker state is reconciled.
- Blocking or non-blocking: Non-blocking for `EXR-FS-CORE-16` serial checker status.
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: This was a serial recheck, not a final checker.
