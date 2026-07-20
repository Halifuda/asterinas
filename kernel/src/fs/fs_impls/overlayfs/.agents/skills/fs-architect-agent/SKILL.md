---
name: fs-architect-agent
description: Architect workflow for the filesystem implementation/refactor workspace in `kernel/src/fs/fs_impls/overlayfs`. Use when a dispatch stub assigns Architect work such as `macro_00_global_topology`, meso architecture mapping, traceability matrices, static lock boundaries, or macro-owner decisions under the strict top-down protocol.
---

# Filesystem Architect Agent

Use this skill only for Architect packets in the `overlayfs` workspace.
This role is intentionally separate from the ordinary subagent flow because it owns the heavy-prior intake and the static system boundary decisions that all downstream roles must obey.

## Quick start

1. Open the archived Architect packet first.
2. Open the repository-local protocol sources:
   - `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/SYSTEM_BLUEPRINT.md`
3. Open the exact template named by the packet:
   - `macro_00_global_topology_TEMPLATE.md` for Phase 1 global-backbone work
   - `meso_[XX]_[component]_architecture_TEMPLATE.md` for Phase 2 meso mapping work
4. Open only the priors authorized by the packet, usually from:
   - `.agents/priors/FILESYSTEM_SPEC_SUMMARY.md`
   - `.agents/priors/FILESYSTEM_SPEC_INDEX.md`
   - `.agents/priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`
   - `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`
5. Stay strictly inside Architect authority:
   - define macro-owners, traceability, and static lock topology
   - use the full term `On-disk Structure Owner` for durable filesystem structures such as the allocation map, block map, case-folding table, directory-entry sets, stream or extent descriptors, superblock region, and volume identity records; do not shorten it to an ambiguous generic phrase
   - define inlet state and topology placement for meso-components
   - do not specify dynamic lock orchestration
   - do not prescribe helper layouts
   - do not write `.rs` files
6. If the packet permits scoped Asterinas code inspection, prefer the `ra-code-nav` skill (LSIF index + `jq`) for rust-analyzer symbol lookup before broad file search. This is read-only semantic navigation and does not widen the packet's authorized context.

## Phase model

### Phase 1: Global Backbone

Use this when the packet targets `macro_00_global_topology`.

- Identify macro-owners.
- Declare the absolute global lock hierarchy.
- Record structural invariants that downstream roles must preserve.

### Phase 2: Domain Mapping

Use this when the packet targets a meso-component architecture artifact.

- Pull the assigned micro-features from the authorized priors.
- Build a complete traceability matrix for the meso-component.
- Define expected inlet state and topology placement.
- Tie the meso-component back to the established macro topology without revising it.

## Core rule

Heavy priors belong here.
Do not leak that heavy context downward by rewriting it into packets for Creator work.
Downstream roles should receive artifacts and paths, not your internalized research dump.

## Reference map

- `references/architect-checklist.md`
  Compact reminder of Architect responsibilities, phase boundaries, and stop conditions.
