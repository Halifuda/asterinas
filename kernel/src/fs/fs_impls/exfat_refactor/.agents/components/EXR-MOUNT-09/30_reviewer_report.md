<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `Reviewing`
- Author: `main-agent`
- Date: `2026-04-05`

## Scope Review

Reviewed `fs.rs` plus the narrow supporting changes in `fat.rs`, `inode.rs`, and `bitmap.rs` for boundary drift, helper justification, publication semantics, and whether the checker-driven local test repair widened the production surface.

## Result

No bounded code-quality edit is needed in this pass.

The current implementation stays within the agreed mount boundary:

- `fs.rs` owns only bootstrap assembly and one-shot publication,
- the accepted `SYSROOT`, `UPCASE`, and `BITMAP` surfaces are consumed rather than rediscovered,
- the synthetic root remains explicit through `ExfatInodeMeta::new_root(...)`,
- the only new helper in production code is `ExfatChain::byte_len(...)`, which is justified by the mount root-size derivation instead of by speculative cross-module accessor growth,
- the checker repair stayed local to `fs.rs` tests and did not add a gratuitous `Debug` surface to `ExfatFs`.

## Notes

- `ExfatAllocationBitmap` and `ExfatInodeMeta` gained `Eq`/`PartialEq` support only so the local mount tests can compare published state against expected values without widening production accessors.
- I did not run build, test, or QEMU commands in this review pass.

## Verdict

Accepted as-is for the bounded review stage. The next pass should rerun the focused local `fs::tests` suite under the normal checker lock before `EXR-MOUNT-09` is accepted.
