<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Reviewing`
- Author: `reviewer`
- Date: `2026-04-04`
- Task packet: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1622-reviewer-duplicate-helpers-scan-packet.md`
- Reviewed implementation:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`

## Review Scope

Performed a bounded duplicate-helper scan across the touched `exfat_refactor` modules, with emphasis on checksum-style helpers, size/range guards, and helper ownership. This pass was report-only and did not run verification commands.

## Findings

No real cross-module duplicate helpers were found in the scanned set. The candidate overlaps below are structurally similar but remain justified by different contracts and owners.

### Candidate overlap

- Severity: `informational`
- Location: `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs:122-127` and `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs:179-187`
- Description: Both helpers use the same rotate-add checksum shape, but they do not share the same contract. `checksum32()` validates the discovered upcase payload, while `verify_primary_boot_region_checksum()` authenticates the boot region and intentionally skips mutable fields in a fixed sector layout. This is an acceptable local parallel, not a duplicate helper.
- Guideline or style principle involved: prefer a single canonical helper only when the owner and contract are the same; keep local helpers when the boundary semantics differ.
- Action taken: None. Reported as a non-duplicate.

### Candidate overlap

- Severity: `informational`
- Location: `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs:312-319` and `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs:88-96`
- Description: `checksum_utf16()` and `ExfatUpcaseTable::name_hash()` both fold UTF-16 inputs with rotate-add arithmetic, but they implement different on-disk contracts. The former reproduces the file-record checksum over raw UTF-16 units; the latter applies table-backed folding before computing the exFAT `NameHash`. This is not a duplicate helper.
- Guideline or style principle involved: boundary hygiene and contract-specific helper ownership.
- Action taken: None. Reported as a non-duplicate.

### Candidate overlap

- Severity: `informational`
- Location: `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs:139-154` and `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs:104-119`
- Description: Both helpers enforce payload size constraints, but they validate different invariants. `minimum_bitmap_byte_size()` derives a geometry-dependent lower bound for the allocation bitmap, while `validate_upcase_table_size()` enforces the upcase table's minimum even-sized payload. The shape is similar, but the owners and meanings are distinct.
- Guideline or style principle involved: keep local guards close to the boundary they protect; do not collapse distinct on-disk invariants into one generic helper without a shared owner.
- Action taken: None. Reported as a non-duplicate.

## Direct Edits

- Created `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/35_reviewer_duplicate_helpers_scan.md`.

## Residual Concerns

- None for this bounded scan. The read set was sufficient to classify the candidate overlaps.

## Recommendation

- Next owner: `checker`
- Reason: this pass found no true duplicate helper that warrants a refactor; the remaining work is to keep the component moving through the existing workflow.
