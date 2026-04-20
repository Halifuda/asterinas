<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Pass Validation Report / Repair Batch: `mount_volume_state_cleanup_02`

*This artifact is the absolute runtime validation record for one Checker pass. It either acts as a final signature that the assigned pass executed its Designer KTest covenants safely, or it outputs an Actionable Repair Batch.*

## 1. Pass Identity

**Checker Pass ID:** `pass_01_mount_volume_state_cleanup_02`
**Pass Kind:** `Creator-Synced`
**Parent Meso-Component:** `meso_01_mount_volume_state`
**Covered Micro-Features:**
- `Boot region validation and parameter load at mount`
- `Allocation bitmap is the free-space truth source`
- `VolumeDirty marks in-flight versus quiesced global state`
- `VolumeFlags also carries media-failure and clear-before-modify state`
- `Up-case Table is the durable case-folding truth source`
- `Mount option defaults and remount mutability boundary`
- `Superblock counters and statfs reflect cached cluster accounting`
- `Asterinas mount lifecycle must eagerly expose root inode and global sync state`
- `Mount-time accounting may fall back to recount under corruption-recovery conditions`
**Creator Pass Artifact(s):**
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_creator.md`

## 2. Test Obligations Executed

**Tests Implemented from `_designer_ktest.md` (Using `#[ktest]` and `#[cfg(ktest)] mod tests`):**
- `mount_volume_state_mount_publishes_root_inode_superblock_and_defaults`: verifies base-case mount publication, root inode availability, default mount options, superblock counters, and accepted Up-case truth state.
- `mount_volume_state_root_and_superblock_reads_are_stable`: verifies repeated root and superblock projections are stable after mount publication.
- `mount_volume_state_recount_fallback_marks_cached_accounting`: verifies recount fallback produces coherent cached accounting.
- `mount_volume_state_preserves_volume_anomaly_flags`: verifies `VolumeDirty`, `MediaFailure`, and `ClearToZero` are preserved as mount-visible anomaly posture.
- `mount_volume_state_rejects_invalid_boot_region`: verifies invalid boot-region data fails before publication with `InvalidOnDiskLayout`.
- `mount_volume_state_rejects_boot_region_device_io`: verifies boot-region read failure maps to `DeviceIo`.
- `mount_volume_state_rejects_inconsistent_allocation_bitmap`: verifies unrecoverable bitmap inconsistency maps to `InconsistentAccounting`.
- `mount_volume_state_rejects_allocation_bitmap_device_io`: verifies Allocation Bitmap media failure maps to `DeviceIo`.
- `mount_volume_state_rejects_invalid_upcase_table`: verifies invalid Up-case Table data fails before naming truth publication.
- `mount_volume_state_remount_allows_discard_and_rejects_immutable_delta`: verifies accepted `discard` remounts linearize and rejected immutable deltas leave prior policy unchanged.

## 3. Lock-Guarded Evaluation Result

*Document the results from the Checker Execution Lock stage. Prefer `.agents/tools/checker_run.sh` and include both the wrapper command and the underlying Docker command(s). If running manually, include the explicit `cargo osdk test` command executed in the Docker environment (`codex-asterinas-dev`).*

- **Reproduce Command**: Preferred wrapper attempt `.agents/tools/checker_run.sh make-kernel --component pass_01_mount_volume_state_cleanup_02 --phase checker` failed before command execution because `.agents/tools/checker_lock.sh` was not executable (`Permission denied`). Manual lock fallback was then used with `bash kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component pass_01_mount_volume_state_cleanup_02 --phase checker --command 'manual: make kernel' --retry-seconds 60 --wait-budget-seconds 0`, followed by `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`.
- **Exact-Name Proof**: No exact-name ktest could execute because the full compile gate failed first. The blocked exact-name ktest set is the ten required `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_*` filters listed in the dispatch packet.
- **qemu-serial.log Scan**: No pass-specific QEMU-backed ktest started, so no valid pass-specific `qemu-serial.log` was produced or archived. The failure is a pre-QEMU Rust compile failure. Compile receipt: `kernel/src/fs/fs_impls/exfat_refactor/.agents/checker-runs/20260420-205640-pass_01_mount_volume_state_cleanup_02-checker-manual-compile/00-make-kernel.log`; lock release receipt: `kernel/src/fs/fs_impls/exfat_refactor/.agents/checker-runs/20260420-205640-pass_01_mount_volume_state_cleanup_02-checker-manual-compile/checker-lock-release.toml`.

## 4. Conclusion (Accepted OR Repair Batch)

### OUTCOME B: ACTIONABLE REPAIR BATCH FOR FOLLOW-UP CREATOR PASS(ES)
- **Status:** **FAIL / BUILD FAILURE**
- **Reproduce Command:** `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`
- **Failed Test:** Full compile gate before exact-name ktests for `pass_01_mount_volume_state_cleanup_02`.
- **Evidence:** `make kernel` fails compiling `aster-kernel` with production errors: `kernel/src/fs/fs_impls/exfat_refactor/boot.rs:237:59` returns `Result<(AllocationBitmapRecord, UpcaseRecord), MountVolumeStateError>` from the `walk_cluster_chain` visitor where `Result<ChainVisitControl, MountVolumeStateError>` is required; `kernel/src/fs/fs_impls/exfat_refactor/boot.rs:321:10` and `kernel/src/fs/fs_impls/exfat_refactor/fat.rs:136:10` call `read_bytes` on `&dyn BlockDevice` without bringing the extension trait `ostd::mm::VmIo` into scope.
- **Actionable Instruction for Follow-Up Creator Pass(es):**
   - **Fix 1:** In `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`, update `scan_root_directory` so the `walk_cluster_chain` closure returns only `Ok(ChainVisitControl::Stop)` when it sees the end-of-directory entry or when both the bitmap and Up-case records are found; leave the existing post-walk `finalize_root_records(bitmap, upcase)` call as the only tuple-returning path.
   - **Fix 2:** In `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`, import `ostd::mm::VmIo` so `read_device_bytes` can call the `read_bytes` extension method on `&dyn BlockDevice`.
   - **Fix 3:** In `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`, import `ostd::mm::VmIo` for the same `read_device_bytes` extension-method call.
   - **Fix 4:** After the production compile fixes, rerun the Checker lane from the full compile receipt, then execute every required exact-name ktest and archive/evaluate each QEMU serial log before claiming acceptance.
