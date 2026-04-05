<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-DIR-10`
- Title: Directory Iteration And Lookup Over Shared Filesystem State
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-DIR-10-DESIGN-20260405-1058`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Checker-owned serial ktests for directory lookup and iteration over mounted shared state.
  - Validation that the canonical upcase-backed name path is used for comparison.
  - Validation that directory scanning stops at end-of-directory and does not turn into mutation.
- Out of scope:
  - Dedicated concurrency tests.
  - Mount bootstrap tests.
  - Page-cache or regular-file read tests.
  - Namespace mutation tests.

## Serial Checker Obligations

- Required checker-owned tests:
  - A mixed-case lookup test that proves the same validated directory record is found through the canonical upcase-backed path.
  - A name-hash mismatch test that proves the lookup path rejects a nonmatching candidate before acceptance.
  - A directory-iteration test that proves `0x00` ends the scan immediately and deleted or free entries are skipped linearly.
  - A root-and-subdirectory coverage test that proves the same directory entry point can scan both kinds of validated directory state from the mounted filesystem object.
  - A separation test that proves no create, unlink, mkdir, rmdir, rename, or file-read behavior is needed to exercise the directory contract.
- Observable properties that must pass:
  - Lookup and iteration both rely on the same canonical candidate policy.
  - Case-insensitive matching uses the loaded upcase-table service.
  - The scan respects the on-disk directory terminator.
  - The tests remain local and do not require any async or concurrency harness.

## Concurrency Checker Obligations

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass:
  - None; the component's contract is read-only and synchronous, and the mount publication contract is already covered by `EXR-MOUNT-09`.

## Acceptance Notes

- Keep the tests focused on directory policy rather than mount bootstrap or inode writeback.
- If a test starts needing mutation helpers, page-cache state, or a second lookup surface, the component boundary has drifted.
