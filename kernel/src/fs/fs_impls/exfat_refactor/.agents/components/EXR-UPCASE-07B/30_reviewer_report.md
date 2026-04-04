<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Reviewing`
- Author: `reviewer`
- Date: `2026-04-04`
- Task packet: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1559-reviewer-packet.md`
- Reviewed implementation:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Review Scope

Checked boundary hygiene, helper discipline, temporary staging surfaces, and invariant expression on the canonical upcase-backed hash surface and its `fileset.rs` consumer path. This pass was report-only and did not run verification commands.

## Findings

### Blocking defect

- Severity: `blocking`
- Location: `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs:177-183`
- Description: `ExfatDentrySet::validate()` still compares `stream_dentry.name_hash` against `checksum_utf16(&raw_name_units)`. That keeps the consumer path on the provisional raw-UTF-16 checksum contract instead of the canonical table-backed `ExfatUpcaseTable::name_hash()` surface, so the component still has overlapping normalization behavior rather than one canonical source of truth.
- Guideline or style principle involved: boundary hygiene, single canonical service surface, avoid overlapping helpers without a justified caller boundary.
- Action taken: None. Report-only pass.

## Direct Edits

- Created `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`.

## Residual Concerns

- The temporary ktest-only builder `from_trusted_metadata()` remains explicitly marked as staging for later writeback ownership, which is acceptable for now.
- The production validation path still needs to be redirected to the canonical table-backed hash service in the repair lane.

## Recommendation

- Next owner: `creator`
- Reason: the production consumer path still needs a code change to replace the raw checksum comparison with the canonical upcase-backed hash contract.
