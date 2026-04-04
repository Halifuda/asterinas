<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-MOUNT-09-DESIGN-20260404-1511`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the checker-owned regressions needed to prove that mount bootstrap consumes validated root facts, seeds the synthetic root inode, and publishes one complete shared filesystem object without rediscovery or partial visibility.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs`
- Helper touch: tests may inspect module-private fields if the mount object keeps them private; no production getter expansion is required for this component

## Required Coverage

### Scenario 1: Happy-path mount assembles one complete filesystem object

- Test intent:
  - Confirm the mount constructor accepts prevalidated superblock facts and root discovery facts and returns one complete shared filesystem object.
- Suggested test shape:
  - Use the standard exFAT fixture, obtain the validated superblock and root discovery aggregate from the accepted loader surfaces, and call the mount constructor.
- Assertions:
  - The mount succeeds.
  - The returned object contains the loaded upcase table, the loaded allocation bitmap, and the root inode seed.
  - No second root scan or lookup step is needed to obtain the mount state.

### Scenario 2: Missing root discovery facts are rejected

- Test intent:
  - Confirm mount owns the completeness boundary for the accepted root discovery aggregate.
- Suggested test shape:
  - Pass a root discovery aggregate missing either the bitmap fact or the upcase fact.
- Assertions:
  - The mount returns an error.
  - No partial filesystem object is published.

### Scenario 3: The synthetic root inode is seeded through the explicit root constructor

- Test intent:
  - Confirm the mount path does not route the root through the ordinary inode constructor.
- Suggested test shape:
  - Inspect the published root seed on the mounted filesystem object.
- Assertions:
  - The root shell uses the reserved root key.
  - The root shell behaves as a directory shell created by the synthetic root path.
  - The ordinary inode constructor is not required for the root seed.

### Scenario 4: Bootstrap failure is atomic

- Test intent:
  - Confirm the mount constructor does not leak partially initialized shared state.
- Suggested test shape:
  - Corrupt the upcase checksum or bitmap self-coverage so one of the dependent loaders fails.
- Assertions:
  - The mount returns an error.
  - No filesystem object with only some of the dependent state is visible to the caller.

### Scenario 5: Mount stays out of lookup and mutation policy

- Test intent:
  - Confirm the mount path is a bootstrap layer, not a namespace engine.
- Suggested test shape:
  - Keep the regression local to mount construction and inspect only the shared-state handoff.
- Assertions:
  - No directory lookup, create, unlink, rename, or page-cache behavior is required to satisfy the mount contract.
  - The mount contract is satisfied by assembly and publication only.

## Observability

- The tests should stay local to `fs.rs`.
- The tests should inspect only the mount result and the error path.
- They should not require page cache, namespace mutation, lookup orchestration, or background tasks.
- They should not introduce a separate helper module unless the mount test block becomes unexpectedly crowded.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves the mount component is a bootstrap-and-publication layer, not a rediscovery or lookup layer. The happy-path mount regression can satisfy that obligation if it starts from the accepted root discovery aggregate and completes without any second root scan.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and verify both of these statements:

1. the mount constructor publishes one complete shared filesystem object from validated inputs,
2. the mount constructor rejects incomplete or malformed root facts without exposing partial shared state or pulling in lookup policy.

