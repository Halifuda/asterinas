<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Dispatch: origin_index_export

**Role ID:** DESIGNER
**Pass Kind:** Meso-Level Dynamic Contract + External Validation Contract
**Component/Task Group:** `origin-index-export`
**Parent Meso-Component:** `origin_index_export`
**Covered Micro-Features:** P2-04, P3-01, P3-02

## 1. Input Context (Read-Only)

Read this packet first, then the role and scheduler protocols:

- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/SYSTEM_BLUEPRINT.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/macro_00_global_topology.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/meso_12_origin_index_export_architecture.md`
- `test/initramfs/src/conformance/xfstests/README.md`
- `test/initramfs/src/conformance/xfstests/overlay/run_list/full.list`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_INDEX.md`

Use only the accepted Architect topology and the local component context needed to produce this meso contract. Do not read or cite `kernel/src/fs/fs_impls/overlayfs/fs.rs` or use any legacy implementation as a design oracle. Do not run builds, tests, QEMU, xfstests, or other dynamic commands.

## 2. Required Artifacts

Write exactly these two files under the matching component directory:

- `kernel/src/fs/fs_impls/overlayfs/.agents/components/origin-index-export/meso_12_origin_index_export_designer_spec.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/origin-index-export/meso_12_origin_index_export_designer_validation.md`

Use these templates:

- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/meso_[XX]_[component]_designer_spec_TEMPLATE.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/meso_[XX]_[component]_designer_validation_TEMPLATE.md`

## 3. Scope and Validation Boundary

The dynamic specification must cover exactly the parent Meso boundary and all covered Micro IDs above. Preserve the accepted Macro lock topology, including same-level instance ordering and lock-neutral reentrant callback boundaries; do not revise ownership, lock levels, or add new architecture.

The validation artifact is xfstests-only and many-to-many. Map every covered Micro ID to upstream xfstests IDs or groups, classify each as `direct`, `combined`, `not-run/unsupported`, or `no upstream coverage`, and record runtime/integration observations. A missing upstream case is a documented limitation, not a reason to add an internal unit test, ktest, filesystem-local test, or test-only helper.

Do not pre-slice Creator or Checker passes, do not design exact new Rust signatures, do not design private helpers, and do not write any .rs file. Stop after the two Markdown artifacts are complete and report their paths plus the Micro ID count.
