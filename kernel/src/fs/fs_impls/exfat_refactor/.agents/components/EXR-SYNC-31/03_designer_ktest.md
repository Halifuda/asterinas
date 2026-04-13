<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-SYNC-31`
- Title: `ExfatFs` sync delegation and flush-ordering coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1304-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Purpose

Define the minimum checker-owned regressions needed to prove that sync remains owned by `ExfatFs`, that inode sync hooks delegate into the same owner-private boundary, and that page-cache writeback does not become a second manager.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs` and `inode.rs` as appropriate
- Helper touch: owner-private test helpers may be added only if they are needed to build filesystem and inode fixtures for delegation checks

## Required Coverage

### Scenario 1: Filesystem sync remains the owner root

- Test intent:
  - Confirm that `FileSystem::sync()` is the single filesystem-wide persistence entry point on `ExfatFs`.
- Suggested test shape:
  - Build a mounted `ExfatFs` fixture.
  - Call `sync()` on a clean filesystem.
  - Repeat the call after any already-supported dirty state has been drained once.
- Assertions:
  - The call returns success.
  - Stable owner-visible snapshot data, such as the superblock projection and subscriber stats, remains unchanged.
  - The call does not depend on control-path policy or a new public writeback service.

### Scenario 2: Inode sync hooks delegate into the same root

- Test intent:
  - Confirm that `Inode::sync_all()` and `Inode::sync_data()` on `ExfatInode` both reach the same filesystem-owned flush-ordering boundary.
- Suggested test shape:
  - Build a regular-file inode fixture from `ExfatFs`.
  - Invoke `sync_all()` and `sync_data()` on the inode in separate runs.
  - Compare the postconditions for stability and success.
- Assertions:
  - Both methods return success.
  - Neither method widens into an independent owner or a separate policy branch.
  - Repeated calls are idempotent once the same dirty state has been drained.

### Scenario 3: Page-cache writeback remains downstream

- Test intent:
  - Confirm that `write_page_async()` stays a downstream persistence seam rather than a second page-cache owner.
- Suggested test shape:
  - Build a dirty regular-file page-cache fixture.
  - Route page writeback through the inode backend and then call filesystem sync.
  - Check that the resulting state is ordered through the same owner-private path.
- Assertions:
  - `write_page_async()` does not require a separate writeback manager.
  - The same filesystem-owned ordering root is used for page writeback and sync.
  - No extra cache-owner abstraction appears in the fixture or the observable result.

### Scenario 4: Repeated sync calls stay idempotent

- Test intent:
  - Confirm that once dirty state has been drained, the next sync call sees the same clean owner boundary.
- Suggested test shape:
  - Publish the smallest available dirty state through buffered write or namespace mutation fixtures.
  - Call `sync()` twice.
  - Inspect the same owner-visible state before and after the second call.
- Assertions:
  - The second call succeeds.
  - The second call does not alter the stable owner snapshot.
  - The second call does not create a new dirty producer or a second flush path.

## Observability

- These tests should inspect filesystem sync return values, inode sync return values, and stable owner-visible snapshot data.
- They should treat `write_page_async()` as a downstream seam, not as a separate public manager.
- They should not add direct-I/O coverage, boot policy coverage, volume-label mutation coverage, or admin ioctl coverage.
- They should not introduce dedicated control-path tests for later rows that are only mentioned as future dirty producers here.

## Minimal Checker Obligation

The checker must include regressions proving that:

- `FileSystem::sync()` remains the filesystem-wide owner root,
- inode `sync_all()` and `sync_data()` are thin delegates into the same boundary,
- page-cache writeback stays downstream to `ExfatFs`,
- and repeated clean sync calls remain idempotent.

## Exit Condition

The ktest plan is complete when a future checker can implement it entirely in local `fs.rs` and `inode.rs` tests and can verify that sync remains a flush-ordering boundary only, without a public writeback manager, a filesystem-global cache service, or control-path policy drift.
