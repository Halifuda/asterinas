<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-FS-CORE-16`
- Title: `ExfatFs` Filesystem Owner Boundary
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1052-checker-serial-packet.md`
- Checked implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Pass kind: `serial`

## Scope of Review

Checked the `ExfatFs` owner skeleton against the designer obligations in `01_designer_core.md` and `03_designer_ktest.md`, the creator record in `10_creator_serial.md`, and the local `FileSystem` trait surface in `kernel/src/fs/vfs/fs_apis/file_system.rs`.

The review covered `ExfatFs::new`, `name()`, `sb()`, `fs_event_subscriber_stats()`, the temporary `root_inode()` seam, the placeholder `sync()`, and the `mod.rs` declarations for `fs` and `inode`. It did not inspect or edit `inode.rs`.

## Test Changes

Added local `#[cfg(ktest)] mod tests` coverage in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`.

- `filesystem_identity_and_super_block_snapshot_are_stable`: builds one owner from the embedded exFAT disk fixture, exercises the VFS `FileSystem` surface, and checks stable `name()` and `sb()` projection.
- `subscriber_stats_and_snapshot_survive_placeholder_sync`: checks stable stats reference identity, successful placeholder `sync()`, subscriber state preservation, and unchanged superblock snapshot.
- `root_inode_temporary_seam_stays_on_file_system_owner`: uses `#[should_panic]` to confirm the current temporary `root_inode()` handoff remains exposed through the `ExfatFs` `FileSystem` seam until `EXR-FS-OPEN-22`.

Each new ktest has a short scenario comment.

Prepared exact filtered suffixes for execution:

- `filesystem_identity_and_super_block_snapshot_are_stable`
- `subscriber_stats_and_snapshot_survive_placeholder_sync`
- `root_inode_temporary_seam_stays_on_file_system_owner`

These suffixes are source-backed by the unique function names in `fs.rs`.

## Findings

### Finding

- Severity: Blocking for executable verification; not an in-scope production-code defect.
- Location: Docker execution environment.
- Description: The required `codex-asterinas-dev` container was not running, so the checker could not run the filtered ktests.
- Violated spec clause or expected behavior: The task packet requires Docker command form `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-test-suffix>'`.
- Reproduction or reasoning: Under the checker lock, `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'` failed with `Error response from daemon: container cdc951c6672504dbbc3568609420f831b7db8e475798d8ca062b3d035c8b64b1 is not running`. `docker ps --format '{{.Names}} {{.Status}}'` returned no running containers.

No in-scope production-code defects were found from source inspection.

## Verified Properties

- `ExfatFs` remains the owner type implementing `FileSystem`.
- `name()` returns the stable `exfat` identity.
- `sb()` returns the stored VFS `SuperBlock` snapshot rather than reparsing disk state.
- `fs_event_subscriber_stats()` returns the owner-owned stats object.
- `root_inode()` keeps the exact required temporary comment: `// Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.`
- `sync()` remains a placeholder returning success and does not introduce flush ordering, inode-cache traversal, bitmap flushing, allocation policy, or dirty-state ordering.
- No helper wrapper, stats shell, root shell, or field-exposing accessor was introduced for this component.
- `mod.rs` includes the `fs` and `inode` module declarations needed for the Wave A wiring.

## Unverified Properties

- The new ktests were not compiled or executed because the required Docker container was not running.
- KVM/TCG mode could not be observed because Docker execution failed before QEMU or `cargo osdk test` started.
- No output-backed proof of executed tests is available. Coverage proof is limited to source-backed exact suffix selection.

## Commands

- `.agents/tools/checker_lock.sh acquire --component EXR-FS-CORE-16 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test filesystem_identity_and_super_block_snapshot_are_stable'; docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test subscriber_stats_and_snapshot_survive_placeholder_sync'; docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_inode_temporary_seam_stays_on_file_system_owner'" --retry-seconds 60 --wait-budget-seconds 1800`
- `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- `docker ps --format '{{.Names}} {{.Status}}'`
- `.agents/tools/checker_lock.sh release`
- `.agents/tools/checker_lock.sh status`

The filtered `cargo osdk test` commands were not run after the Docker environment failure.

## Recommendation

- Next owner: main-agent
- Reason: Provide or restart the required `codex-asterinas-dev` container, then rerun the three exact filtered ktests under the checker lock.
- Blocking or non-blocking: Blocking for serial executable verification; non-blocking for source-level handoff if the main agent accepts an environment-blocked checker result.
- This was the required serial checker pass, but executable verification is incomplete due to the environment failure.
