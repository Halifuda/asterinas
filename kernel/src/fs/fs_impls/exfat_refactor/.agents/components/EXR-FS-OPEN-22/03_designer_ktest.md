<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` Mount/Open Sequencing Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1300-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`

## Scope

- In scope:
  - Define the checker-owned regression coverage for mount/open sequencing.
  - Validate root publication, publication ordering, and the disappearance of the indefinite root seam.
  - Keep tests focused on the `ExfatFs` owner boundary and not on later directory or write-side behavior.
- Out of scope:
  - Directory mutation, allocator mutation, data-path behavior, and sync semantics.
  - Tests for later `EXR-DIR-OPS-23`, `EXR-ALLOC-27`, `EXR-WRITE-30`, or `EXR-SYNC-31`.
  - Any test that depends on a new mount object, scanner owner, or fake root carrier.

## Checker Coverage

### 1. Root Publication Regression

- Goal:
  - Confirm that the root inode is published by `ExfatFs` and is returned as the same canonical handle on repeated access.
- What it should assert:
  - Repeated root access after mount returns the same root owner state rather than a fresh inode shell.
  - The canonical root handle is visible only after the mount/open sequence has completed.
- Why it matters:
  - This proves the temporary `root_inode()` seam has a named exit path.

### 2. Sequencing Regression

- Goal:
  - Confirm that mount/open sequencing consumes prerequisites in the expected owner-local order.
- What it should assert:
  - The mount path cannot publish a root inode before the upcase state, bitmap state, and root-directory discovery prerequisites are satisfied.
  - The discovery path is driven through `DirectoryEngine` rather than a separate scanner owner.
- Why it matters:
  - This prevents future creator work from reordering the mount path opportunistically.

### 3. Cache-Backed Reuse Regression

- Goal:
  - Confirm that the opened-inode cache remains the reuse boundary for the root special case.
- What it should assert:
  - A second publication attempt for the same root resolves to the canonical handle instead of a duplicate inode shell.
  - The root special case remains distinct from ordinary keyed entries.
- Why it matters:
  - This keeps the root public path aligned with the owner-private cache model.

### 4. Seam Removal Regression

- Goal:
  - Confirm that the old temporary seam is no longer required once the mount/open path exists.
- What it should assert:
  - There is no indefinite placeholder path for `root_inode()` after the mount/open implementation lands.
  - The owner state itself provides the ready root answer.
- Why it matters:
  - This protects the owner boundary from drifting back into a staging seam.

## Suggested Test Shape

- A mount-path test that creates a filesystem owner with valid boot facts and verifies the ready root is published.
- A reuse test that exercises repeated root access and checks canonical identity.
- A prerequisite-order test that fails if discovery or publication is attempted before the required owner state is ready.
- A seam regression that ensures the temporary root path is gone once the unit is implemented.

## Acceptance Notes

- The checker should stay inside the `ExfatFs` boundary and not start validating later directory lookup or write-side behavior here.
- The checker should treat root publication and mount/open readiness as the only user-visible outcomes for this component.
- The checker should reject any test plan that requires a separate mount owner or scanner owner to pass.
