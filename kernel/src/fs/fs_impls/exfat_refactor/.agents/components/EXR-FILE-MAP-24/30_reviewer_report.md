<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Role: `reviewer`
- Date: `2026-04-12`
- Packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1107-reviewer-packet.md`

## Scope

- Reviewed the landed owner-private mapping helpers and checker-added local ktests in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`.
- Checked the current shape against the accepted designer boundary, the creator temporary-surface record, and the checker's executable evidence.

## Findings

- No findings.

## Boundary Review

- `map_physical_file_range()`, `mapping_chain()`, `mapping_cluster_size()`, `physically_backed_end()`, and `physically_mappable_byte_count()` remain owner-private helper logic on `ExfatInode`; they do not widen into buffered-read policy, zero-fill ownership, page-cache ownership, or a standalone mapping service.
- `PhysicalFileRange` is a module-local record shape rather than a leaked service surface. That is acceptable for now because it packages one translation result for the later inode-owned read path without exposing a new public owner boundary.
- The explicit `&dyn BlockDevice` and `&ExfatSuperBlock` arguments remain acceptable as a temporary seam for this row because the packet and creator artifact already record that `inode.rs` cannot yet source traversal context through a narrower owner-approved accessor without widening into `fs.rs`.
- The likely removal condition for that temporary seam remains the later read-path owner in `EXR-READ-OPS-25`, once `ExfatInode` can consume mapping helpers through an accepted inode-owned traversal context.

## Code-Quality Notes

- The helper split is small and top-down readable: regular-file gate, explicit empty/unbacked fast return, geometry verification, chain walk, then single-cluster span derivation.
- The checker-added fixture helpers and four `file_mapping_*` ktests stay local to `inode.rs` and exercise the intended translation-only contract without widening the component boundary.
- I did not see a local maintainability issue that justified a bounded production edit inside this packet.

## Production Edit Summary

- Production code changed in this reviewer pass: `No`.
- Direct edits in this reviewer pass: reviewer artifact only.

## Recommendation

- Reviewer result: `Pass`
- Next owner: main-agent
- Reason: the landed mapping helpers match the current designer boundary, the temporary surfaces are documented and still justified, and the checker already supplied the required executable evidence.
