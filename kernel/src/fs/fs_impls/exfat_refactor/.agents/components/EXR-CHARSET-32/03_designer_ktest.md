<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-CHARSET-32`
- Title: `ExfatFs` Charset And External-Name Conversion Boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1306-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the charset boundary is a real `ExfatFs`-owned conversion service and not a generic Unicode helper or a hidden second policy layer.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: primarily local to `fs.rs`, with legacy consumer regressions in `inode.rs` if needed to prove accepted read-side callers now consume the same owner boundary
- Helper touch: owner-private test helpers inside `fs.rs` and narrow read-side fixtures in `inode.rs` only if needed to keep the tests readable

## Required Coverage

### Scenario 1: Name conversion accepts valid UTF-8 text

- Test intent:
  - Confirm a valid VFS-visible `&str` name is converted into a validated UTF-16 converted-name value.
- Assertions:
  - The conversion succeeds for a representative exFAT-safe name.
  - The returned value is UTF-16 text plus length only.
  - No hash is attached by the charset boundary.

### Scenario 2: Label conversion accepts valid UTF-8 text

- Test intent:
  - Confirm a valid VFS-visible `&str` volume label is converted into a validated UTF-16 converted-label value.
- Assertions:
  - The conversion succeeds for a representative exFAT-safe label.
  - The returned value is UTF-16 text plus length only.
  - No namespace-mutation behavior is introduced by the label path.

### Scenario 3: Invalid or overlong input is rejected

- Test intent:
  - Confirm malformed or overlong external text fails before publication of a converted value.
- Assertions:
  - The conversion rejects invalid input with an error.
  - No partial UTF-16 value is exposed.
  - The same invalid shape remains rejected on repeated calls.

### Scenario 4: Repeated conversion is stable

- Test intent:
  - Confirm the same valid input produces the same validated output shape on repeated calls.
- Assertions:
  - Repeating conversion for the same input yields the same validated converted value.
  - The result is stable for the same mounted filesystem state.
  - The test does not depend on fold/hash behavior.

### Scenario 5: Legacy lookup consumer uses the filesystem-owned conversion boundary

- Test intent:
  - Confirm the existing `lookup` path no longer performs ad hoc UTF-16 conversion in `inode.rs` and instead consumes the charset owner before folding and hashing.
- Assertions:
  - A representative lookup succeeds for a valid visible `&str` name.
  - The lookup path still preserves case-equivalent matching behavior through the installed upcase table.
  - The creator did not leave a second conversion policy local to `lookup`.

### Scenario 6: Legacy readdir consumer uses the filesystem-owned visible-name decode

- Test intent:
  - Confirm the existing `readdir_at` path projects validated UTF-16 record names through the `ExfatFs` charset boundary rather than a local `String::from_utf16()` helper.
- Assertions:
  - A representative directory entry is emitted as the expected visible name.
  - Malformed UTF-16 record names are rejected through the filesystem-owned decode boundary.
  - The read-side path does not reopen a second name-conversion owner in `inode.rs`.

## Observability

- These tests should inspect only the charset boundary on `ExfatFs`.
- They should consume the validated converted value or visible-name decode indirectly through filesystem methods rather than testing a generic helper directly.
- They should not introduce namespace mutation, label mutation, or upcase-table canonicalization coverage.
- No dedicated concurrency tests are required beyond repeated-call stability.

## Minimal Checker Obligation

The checker must include a regression that proves:

- valid external names convert to validated UTF-16 name values,
- valid external labels convert to validated UTF-16 label values,
- accepted lookup and readdir consumers now use the same owner boundary for visible name conversion,
- malformed or overlong input is rejected before publication,
- and repeated conversion is deterministic for the same mounted filesystem state.

## Exit Condition

The ktest plan is complete when a future checker can verify that `ExfatFs` owns both visible-name encode and decode behavior for exFAT without reopening namespace mutation, volume-label mutation, or a generic text subsystem boundary.
