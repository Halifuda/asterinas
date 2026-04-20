<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `mount_volume_state_cleanup_04_followup`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_01_mount_volume_state_cleanup_04_followup`
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

- **Naming Conventions:** Accepted in the scoped follow-up. The legacy `read_le_*` helper family is absent from the reviewed production and ktest support files, leaving direct fixed-width `from_le_bytes(...)` decoding at the call sites.
- **Imports:** Accepted as-is. `ondisk.rs` now pulls its ktest compatibility entrypoints from `test_support`, and the concern-split `test_support/` tree keeps imports local to each support file.
- **Formatting:** Accepted as-is for this bounded follow-up; no reviewer formatting edits were required.
- **Doc Comments:** Accepted in scope. No new undocumented production seam was introduced by cleanup_04, and the existing mount-only TODO seams in `fs.rs` / `inode.rs` remain unchanged.

| Quality Prior Area | Evidence / Line-Level Finding | Action |
|--------------------|-------------------------------|--------|
| `Error handling` | The scoped production mount path still propagates `MountVolumeStateError`, and this cleanup only removes duplicate decode wrappers plus relocates ktest support topology. | Accepted |
| `Visibility` | `mod ondisk;` and `mod test_support;` remain gated by `#[cfg(ktest)]` in `mod.rs`, so the compatibility shim stays outside production ownership flow. | Accepted |
| `Arithmetic / overflow` | The reviewed decode paths use direct `u16` / `u32` / `u64::from_le_bytes(...)` and preserve the existing checked arithmetic around mount geometry and cluster accounting. | Accepted |
| `Lock / RAII readability` | No mount publication, lock ordering, or RAII shape changed in the scoped production files; the follow-up only confirms structural cleanup outcomes. | Accepted |

## 2. Independent Entity Census & Helper Legality Sign-Off

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C) and against its claimed owner/module boundary.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| Legacy `read_le_*` wrapper families in `boot.rs`, `fat.rs`, `bitmap.rs`, `upcase.rs`, and the deleted flat diagnostics surface | No surviving symbols found in the scoped tree | Yes | Former owner-local parse helpers and former flat ktest diagnostics namespace | Accepted as removed. The cleanup leaves direct `from_le_bytes(...)` decoding in place instead of thin wrappers. | Accepted |
| `test_support` concern split (`mount_diagnostics`, `boot_region`, `root_directory`, `bitmap`, `upcase`) | Yes | Yes | Dedicated `#[cfg(ktest)]` support hierarchy under `kernel/src/fs/fs_impls/exfat_refactor/test_support/` | Accepted. The support surface is split by concern instead of accumulating in a flat catch-all file. | Accepted |
| `test_support::load_validated_mount` and `test_support::diagnose_invalid_on_disk_layout_gate` | Yes | Yes | Ktest-only compatibility entrypoints exported from `test_support/mod.rs` | Accepted. The compatibility helpers now live under the dedicated test-support owner instead of a flat sibling module. | Accepted |
| `ondisk.rs` ktest shim | Yes | Yes | `#[cfg(ktest)]` compatibility shim declared only from `mod.rs` and re-exporting its compatibility functions from `test_support` | Accepted. The shim remains outside production flow and no longer targets a flat `diagnostics.rs` file. | Accepted |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** Accepted for the cleanup_04 follow-up scope. Reviewer found no surviving `read_le_*` wrappers and no stray flat `diagnostics.rs` namespace; the Creator's reported `test_support/` split matches the current tree.
- **Owner / Module Placement:** Accepted. Direct little-endian decoding now stays at the local parse sites, and non-trivial `#[cfg(ktest)]` support lives under the dedicated `test_support/` hierarchy split by concern.
- **Temporary Facades / Dead Variants:** Accepted. Reviewer found no new dead production facade in scope, and `ondisk.rs` remains a ktest-only compatibility shim with no production consumers in the reviewed module tree.

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** No new temporary production seam was introduced by cleanup_04. The existing mount-only TODO seams in `fs.rs` and `inode.rs` are unchanged, and the remaining `ondisk.rs` compatibility layer is still isolated behind `#[cfg(ktest)]` rather than participating in production ownership flow.
- **Edits Made:** None.

## 4. Edit Scope Classification

- **Reviewer Edit Scope:** `No edits`
- **Why This Scope Is Safe:** The follow-up review is a bounded static confirmation only. The required cleanup goals are satisfied in the current tree, so no line-level reviewer repair was needed and no final Checker rerun is implied by this artifact.

## 5. Final Verdict

- **APPROVED (LINE-LEVEL ONLY; FINAL CHECKER SKIPPABLE)**: The cleanup_04 follow-up satisfies both targeted structural gates. Duplicated `read_le_*` wrappers are gone, the flat `diagnostics.rs` namespace has been replaced by the dedicated `test_support/` hierarchy, and the remaining `ondisk.rs` shim stays ktest-only while sourcing its compatibility exports from `test_support`.
