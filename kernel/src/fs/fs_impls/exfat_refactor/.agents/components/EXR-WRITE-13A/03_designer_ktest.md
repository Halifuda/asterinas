<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-WRITE-13A`
- Title: Writable Regular-File Allocation Growth And Metadata Publication
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-WRITE-13A-DESIGN-20260405-1224`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the checker-owned regressions needed to prove that writable regular-file growth reserves additional clusters, publishes the enlarged chain and allocation boundary atomically, and keeps valid-data advancement separate for later buffered-write work.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs` and `inode.rs` as needed for the canonical growth surface
- Helper touch: tests may inspect module-private growth or publication state if the canonical surface keeps it private; no production getter expansion is required for this component

## Required Coverage

### Scenario 1: Contiguous growth publishes a larger allocation boundary

- Test intent:
  - Confirm a contiguous writable regular file can grow by reserving more clusters and publishing the larger allocation boundary.
- Suggested test shape:
  - Use a regular-file fixture with a contiguous chain, request a larger length that requires additional clusters, and invoke the canonical growth surface.
- Assertions:
  - The growth succeeds.
  - The file's allocation boundary increases to the requested length.
  - The chain still describes one coherent file image after growth.
  - The valid-data boundary remains at its pre-growth value.

### Scenario 2: Growth that extends the chain publishes the accepted chain facts

- Test intent:
  - Confirm a growth that needs chain extension uses the accepted chain boundary rather than inventing a second publication path.
- Suggested test shape:
  - Use a writable regular-file fixture where the new allocation must add clusters beyond the current chain extent.
- Assertions:
  - The growth succeeds.
  - The newly allocated chain segment is visible through the canonical inode or file-state surface.
  - The result reflects the accepted chain mode and the updated allocation boundary together.

### Scenario 3: Valid-data length stays separate from allocation growth

- Test intent:
  - Confirm allocation growth does not also claim that newly allocated bytes are initialized.
- Suggested test shape:
  - Grow a writable regular file without performing buffered writes.
- Assertions:
  - The allocation boundary grows.
  - The valid-data boundary does not advance.
  - Later buffered-write work still has a distinct boundary to advance.

### Scenario 4: Non-regular-file inputs are rejected

- Test intent:
  - Confirm the growth surface does not accept directories or the reserved root shell.
- Suggested test shape:
  - Feed the growth surface a directory shell or the synthetic root shell.
- Assertions:
  - The growth returns an error.
  - No allocation bitmap or chain publication is exposed for the rejected input.

### Scenario 5: Allocation failure is atomic

- Test intent:
  - Confirm a failed reservation or chain-link step does not leak partial growth.
- Suggested test shape:
  - Inject a bitmap or allocation failure at the point where the growth would need additional clusters.
- Assertions:
  - The growth returns an error.
  - The inode-visible allocation boundary remains unchanged.
  - The chain and bitmap do not expose a partially grown result.

## Observability

- Use assertion macros only.
- Keep each test focused on one growth rule or publication boundary.
- Prefer explicit fixture names over inline construction when the setup would obscure the growth rule under test.
- Do not inspect implementation-private fields except through the canonical growth surface needed to validate the contract.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves allocation growth does not advance the valid-data boundary. That regression is the guardrail that keeps buffered-write initialization in a later component.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and verify both of these statements:

1. writable regular-file growth can publish a larger allocation boundary from validated inputs,
2. initialized-length advancement remains separate from allocation growth and stays owned by later write-side work.
