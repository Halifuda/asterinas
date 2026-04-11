<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` Upcase Table Validation, Folding, And Name Hash
- Status: `Specified`
- Author: designer
- Date: 2026-04-10

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the mounted filesystem owner validates the upcase table once, folds UTF-16 names through that table, and computes exFAT name hashes from the folded bytes.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Valid and invalid table publication are distinguished

- Test intent:
  - Confirm that the owner accepts a well-formed upcase table and rejects malformed size or checksum data.
- Suggested test shape:
  - Exercise the owner installation path with a small synthetic valid table fixture and one or more malformed fixtures.
- Assertions:
  - A valid candidate is published once and becomes the source of later folding calls.
  - A candidate with the wrong size is rejected.
  - A candidate with the wrong checksum is rejected.
  - Rejection leaves the owner state unchanged.

### Scenario 2: Folding uses the installed volume table

- Test intent:
  - Confirm that folding is derived from the installed upcase table and not from a generic text helper or locale rule.
- Suggested test shape:
  - Fold at least one mixed-case UTF-16 name through the owner method twice and compare the results.
- Assertions:
  - Repeated folds of the same input return the same canonical units.
  - A case-variant input folds to the same canonical result as its uppercase form.
  - The result is stable for the same mounted filesystem state.

### Scenario 3: Name hash is computed from folded bytes

- Test intent:
  - Confirm that the name-hash service follows the exFAT algorithm over folded UTF-16 bytes.
- Suggested test shape:
  - Compute hashes for two case-variant names that fold to the same canonical units, then compute a second pair that should remain distinct.
- Assertions:
  - Case-equivalent names produce the same hash after folding.
  - Distinct folded names do not collapse to the same hash in the fixture.
  - The hash is unchanged when recomputed on the same folded input.

## Observability

- These tests should use the owner-facing folding and hashing services, not raw internal table fields.
- They should not require directory traversal, mount sequencing, inode work, or bitmap work.
- They should keep any synthetic table fixture as small as possible while still exercising validation, folding, and hashing.

## Minimal Checker Obligation

The checker must include one explicit regression that proves the owner rejects a malformed upcase table before any later folding or hashing call can use it.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage in `fs.rs` and verify that valid publication, deterministic folding, and spec-based name hashing all rely on the same installed upcase table.
