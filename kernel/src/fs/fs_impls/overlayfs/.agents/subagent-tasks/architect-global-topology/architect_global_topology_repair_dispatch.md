<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Repair Dispatch

**Role ID:** ARCHITECT
**Pass Kind:** Macro Backbone + complete Meso Mapping repair
**Component/Task Group:** `architect-global-topology`
**Parent Meso-Component:** N/A
**Covered Micro-Features:** Preserve the complete formal set: `P0-01` through `P0-18`, `P1-01` through `P1-37`, `P2-01` through `P2-17`, and `P3-01` through `P3-09` (81 IDs total).

## 1. Input Context (Read-Only)

Read this repair packet first, then the assigned role protocol and scheduler protocol. Review the existing artifacts and the same staged priors used by the original packet:

- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/macro_00_global_topology.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/meso_*_architecture.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_INDEX.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`

Do not read or cite `kernel/src/fs/fs_impls/overlayfs/fs.rs` or any other legacy implementation as a design oracle. Do not run builds, tests, QEMU, xfstests, or other dynamic commands.

## 2. Repair Scope and Required Edits

Repair the existing Macro artifact and Meso architecture maps in place under:

`kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/`

Do not produce Designer, Creator, Checker, Reviewer, or pass-slicing artifacts. The repair must address every item below:

1. **Same-level lock ordering:** The Macro topology must fully define a deterministic static total order for multiple instances of any same-level lock domain that can be acquired together, especially dual-parent `DIR` locks for rename. Do not defer that rule to the Designer. If `UPPER` is per-directory/object rather than one mount-wide domain, define its instance granularity and same-level ordering too. An immutable identity rule such as a justified `Arc::as_ptr()` order is acceptable; document the chosen rule and its scope.
2. **UPPER ownership and granularity:** Replace the vague “Mutex-compatible sleep boundary” wording with an explicit overlay-owned lock domain, owner, instance granularity, and BIO/reentrancy constraint. Keep the inventory's hard rule that `UPPER` crosses BIO and must use `ostd::sync::Mutex`.
3. **Primary-owner consistency:** Resolve the `P0-04` root construction split. Its Macro primary owner, Meso primary row, responsibility, and static boundary must agree; a collaborator may construct carriers only if the artifact clearly preserves one primary feature owner and does not say the feature “belongs” to another Meso.
4. **Mount-level policy ownership:** Re-evaluate `P1-19` credential stashing/override, `P3-07` override-creds option semantics, and `P2-11` UUID/fsid modes. They are mount/layer identity concerns, not automatically `OverlayInode` concerns. Move them to a coherent mount-owned Meso or explicitly correct the Macro-Owner and interaction boundaries. Also give `P0-18` read-only enforcement one explicit primary policy owner while preserving its cross-cutting invariant over all mutating surfaces.
5. **Legacy citation removal:** Remove the `overlayfs/fs.rs` citation from the P1-37 architecture map and base that row only on the accepted Asterinas integration prior and other allowed staged priors.
6. **Cross-artifact consistency:** Every Meso static inlet/topology statement must be compatible with the repaired Macro topology. No Meso may refer to a lock order, instance order, or owner that the Macro artifact leaves undefined. Preserve the Architect prohibition on dynamic execution paths, helper design, concrete new signatures, and Creator/Checker pass slicing.

## 3. Acceptance Checks Before Returning

- Keep the exact formal 81 IDs, with one primary Meso assignment each; no gaps, duplicates, or historical `P3-10`.
- Keep the Macro template sections and every Meso map's required sections populated.
- Keep the xfstests-only boundary explicit and do not add any ktest or filesystem-local test surface.
- Return the exact changed artifact paths, the final Meso count, the formal ID count, and a concise disposition for each of the six repair items above.
