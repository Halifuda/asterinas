<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `mount_volume_state_cleanup_01`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_01_mount_volume_state_cleanup_01`
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

- **Naming Conventions:** Accepted; the cleanup split keeps descriptive owner-local names and the removed `DirectoryBootstrap` surface does not survive under a trivial rename.
- **Imports:** Rejected at the structural boundary; production `fs.rs` still routes mount-state imports through `ondisk.rs` instead of the new owner-local modules.
- **Formatting:** Accepted; no reviewer formatting edits were needed for this bounded cleanup review.
- **Doc Comments:** Accepted; the previously added exit-plan TODO comments remain present and readable.

| Quality Prior Area | Evidence / Line-Level Finding | Action |
|--------------------|-------------------------------|--------|
| `Error handling` | No new cleanup-specific `.unwrap()` / `.expect()` surface was introduced by the split modules under review. | Accepted |
| `Visibility` | `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` remains a live production compatibility facade because `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` still imports boot/bitmap/upcase carriers through it. | Rejected |
| `Arithmetic / overflow` | The split owner-local modules preserve checked arithmetic on cluster math, stream bounds, and cached accounting paths. | Accepted |
| `Lock / RAII readability` | The cleanup keeps pre-publication I/O and published-state lock scopes unchanged; no new lock-order drift was introduced. | Accepted |

## 2. Independent Entity Census & Helper Legality Sign-Off

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C) and against its claimed owner/module boundary.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| `bitmap` | Yes | Yes | Allocation Bitmap owner-local module | Accepted; real owner-local split exists in `bitmap.rs`. | Accepted |
| `boot` | Yes | Yes | Boot-region / mount-bootstrap owner-local module | Accepted; real owner-local split exists in `boot.rs`. | Accepted |
| `fat` | Yes | Yes | FAT / cluster-chain owner-local module | Accepted; real owner-local split exists in `fat.rs`. | Accepted |
| `upcase` | Yes | Yes | Up-case owner-local module | Accepted; real owner-local split exists in `upcase.rs`. | Accepted |
| `ValidatedMount` | Yes | Yes | Boot-owned bootstrap transit carrier | Accepted; it now lives under `boot.rs` instead of the former catch-all file. | Accepted |
| `ondisk` compatibility surface | Yes | No | Thin compatibility facade only | Vetoed for this cleanup gate: the facade is still on the production path via `fs.rs`, so wrong-owner placement was not fully retired. | REJECT back to Creator for structural cleanup |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** All Creator-listed cleanup-introduced production entities reviewed in the split files are present. Reviewer did not find a surviving `DirectoryBootstrap` or a trivial renamed replacement for it.
- **Owner / Module Placement:** Rejected. `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` still imports `AllocationBitmapRecord`, `BootRegion`, `UpcaseTable`, `VolumeAnomalyState`, and `load_validated_mount` through `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`, so the cleanup leaves a live production compatibility layer under the old catch-all owner boundary.
- **Temporary Facades / Dead Variants:** The dispatcher-only `RootInode` / `SuperBlock` / `Flags` operation-outcome branches are gone, and no dead replacement facade was found there. The blocking structural issue is the still-live `ondisk.rs` production surface.

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** The mount-only `sync()` seam, root inode seam, and deferred registration seam still carry explicit exit-plan comments. The remaining `ondisk.rs` compatibility facade does not need a new exit-plan comment to fail this gate; it fails because it is still active in production ownership flow.
- **Edits Made:** None.

## 4. Edit Scope Classification

- **Reviewer Edit Scope:** `Rejected without edits due structural issues`
- **Why This Scope Is Safe:** The unresolved issue is module-boundary topology, not a line-level style defect. The packet requires routing that debt back to Creator instead of rewriting the structure in Reviewer.

## 5. Final Verdict

- **REJECTED (STRUCTURAL QUALITY CLEANUP REQUIRED)**: The cleanup removes `DirectoryBootstrap` and the dead dispatcher-only branches, but it does not fully retire the agreed wrong-owner / catch-all debt because `ondisk.rs` remains a live production compatibility surface.
