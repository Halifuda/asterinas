<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report Template

## Metadata

- Component ID:
- Title:
- Status: `SerialChecked`, `ConcurrencyChecked`, or `FinalChecked`
- Author:
- Date:
- Task packet:
- Checked implementation:
- Pass kind: `serial`, `concurrency`, or `post-review final`

## Scope of Review

Describe what code, tests, and spec clauses were checked.

## Test Changes

List any tests the checker added, updated, or concluded were still missing.
State where the tests now live if they were relocated for readability.
State whether each checker-owned `#[ktest]` now has a short scenario comment when applicable.

## Findings

Use one entry per issue.

### Finding

- Severity:
- Location:
- Description:
- Violated spec clause or expected behavior:
- Reproduction or reasoning:

Repeat the finding block as needed.

## Verified Properties

List the properties that were positively confirmed.

## Unverified Properties

List the properties the checker could not validate.

## Recommendation

- Next owner:
- Reason:
- Blocking or non-blocking:
