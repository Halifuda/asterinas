<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `mount_volume_state_cleanup_02`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_01_mount_volume_state_cleanup_02`
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

- **Naming Conventions:** Accepted; the surviving owner-local production symbols remain descriptive and keep the expected boolean prefixes.
- **Imports:** Fixed direct production free-function imports in `fs.rs`, `boot.rs`, `bitmap.rs`, and `upcase.rs` so free helpers are referenced through their parent owner modules, matching the repository import prior.
- **Formatting:** Accepted after the import-hygiene edits; no topology or control-flow rewrite was needed.
- **Doc Comments:** Accepted; the previously added exit-plan TODO comments for the mount-only seams remain present and readable.

| Quality Prior Area | Evidence / Line-Level Finding | Action |
|--------------------|-------------------------------|--------|
| `Error handling` | No new cleanup-specific production `.unwrap()` / `.expect()` surface was introduced, and the owner-local helper families continue to propagate on-disk and device errors via `Result`. | Accepted |
| `Visibility` | `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` is no longer on the production path and now remains gated behind `#[cfg(ktest)]` only. | Accepted |
| `Arithmetic / overflow` | The surviving boot, FAT, bitmap, and Up-case helpers continue to use checked arithmetic on geometry, offsets, and cached accounting paths. | Accepted |
| `Lock / RAII readability` | The cleanup keeps the pre-publication I/O boundary, published-state lock order, and remount write-lock scope unchanged. | Accepted |

## 2. Independent Entity Census & Helper Legality Sign-Off

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C) and against its claimed owner/module boundary.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| `ondisk` compatibility facade | Yes | Yes | Test-only compatibility shim under `mod.rs` `#[cfg(ktest)]` | Accepted; the packet-required production retirement is complete because only ktest references remain. | Accepted |
| `PublishedMountState`, `AllocatorState`, `ExfatFs` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Published filesystem owner in `fs.rs` | Accepted; these remain the proper owner-local publication carriers and do not route through `ondisk.rs`. | Accepted |
| `ExfatMountOptions`, `MountVolumeStateTarget`, `MountVolumeStateOperation`, `MountVolumeStateOutcome`, `MountVolumeStateError`, `mount_volume_state`, `mount_candidate`, `remount_published` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Mount-state meso owner in `fs.rs` | Accepted; no dead production dispatch branch was reintroduced, and the surviving free helpers are the meso entry and its two bounded state-transition paths. | Accepted |
| `ExfatInode` | Yes | No (`cleanup_02` introduced none; this predates this pass) | Root inode owner in `inode.rs` | Accepted; still a single owner-local trait carrier, not a loose helper family. | Accepted |
| `BootRegion`, `VolumeAnomalyState`, `ValidatedMount` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Boot / mount-bootstrap owner in `boot.rs` | Accepted; the carriers remain grouped under the boot owner boundary rather than a neutral aggregator. | Accepted |
| `load_validated_mount`, `validate_stream_record` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Boot / mount-bootstrap owner in `boot.rs` | Accepted; this surviving free-helper family stays tightly owner-local and now gets referenced through `boot::...` rather than direct free-function imports. | Accepted |
| `AllocationBitmapRecord`, `count_used_clusters`, `parse_bitmap_record` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Allocation-bitmap owner in `bitmap.rs` | Accepted; the helper pair remains narrow, owner-local, and specifically tied to the bitmap record/type boundary. | Accepted |
| `UpcaseTable`, `UpcaseRecord`, `load_upcase_table`, `parse_upcase_record` | Yes | No (`cleanup_02` introduced none; these predate this pass) | Up-case owner in `upcase.rs` | Accepted; the helper pair remains owner-local and is not exposed through a catch-all facade. | Accepted |
| `ChainVisitControl`, `FatChainStep`, `FatReader`, `walk_cluster_chain` | Yes | No (`cleanup_02` introduced none; these predate this pass) | FAT / chain-traversal owner in `fat.rs` | Accepted; the traversal helper family stays under the FAT owner module with a clear stateful owner type (`FatReader`). | Accepted |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** The `cleanup_02` Creator report is accurate for this pass: Reviewer found no cleanup-specific newly introduced production entity omitted from the census. The surviving production surface audited here predates `cleanup_02` and was rechecked under the packet's stricter wave-local rule.
- **Owner / Module Placement:** Accepted. Production `fs.rs` now imports boot / bitmap / Up-case carriers directly from their owner-local modules, while `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` is restricted to `#[cfg(ktest)]` compatibility only. The surviving production free-helper families remain under clear owner modules (`boot.rs`, `bitmap.rs`, `upcase.rs`, `fat.rs`) instead of a live catch-all production facade.
- **Temporary Facades / Dead Variants:** Accepted. The earlier dispatcher-only production read branches remain removed, and Reviewer found no surviving dead production facade or variant reopened by this cleanup.

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** The mount-only `sync()` seam in `fs.rs` and the root `readdir_at()` / `lookup()` seams in `inode.rs` still carry explicit exit-plan TODO comments. The remaining `ondisk.rs` shim is test-only, so it no longer participates in the production seam boundary that caused the prior rejection.
- **Edits Made:** Narrow, non-functional import-hygiene edits only in `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`, `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, and `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`.

## 4. Edit Scope Classification

- **Reviewer Edit Scope:** `Line-level non-functional edits only`
- **Why This Scope Is Safe:** The direct edits only qualify existing helper calls through their parent modules to match repository import rules. No data flow, visibility boundary, lock scope, or control-flow behavior changed.

## 5. Final Verdict

- **APPROVED (LINE-LEVEL ONLY; FINAL CHECKER SKIPPABLE)**: The production `ondisk.rs` compatibility boundary is retired, the surviving owner-local helper families pass the wave-local structural audit, and the review edits are line-level and non-functional only.
