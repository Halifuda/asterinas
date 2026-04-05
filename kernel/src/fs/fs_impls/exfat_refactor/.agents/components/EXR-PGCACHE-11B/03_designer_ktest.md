<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-PGCACHE-11B`
- Title: Page-Cache Backend Integration For Regular Files
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-PGCACHE-11B-DESIGN-20260405-1134`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the checker-owned regressions that prove the exFAT page-cache backend stays tied to visible file length, uses the accepted placement boundary, and does not absorb buffered read policy.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs` and `inode.rs` as needed for the backend attachment surface
- Helper touch: tests may inspect module-private backend or runtime fields if required by the canonical attachment surface; no production getter expansion is required for this component

## Required Coverage

### Scenario 1: Backend page count tracks visible length

- Test intent:
  - Confirm the backend-visible page count is derived from `valid_data_length`, not from allocated length or chain extent.
- Suggested test shape:
  - Build one regular-file fixture where visible length is smaller than allocated length and one where it is page-aligned.
- Assertions:
  - The backend reports `ceil(valid_data_length / PAGE_SIZE)`.
  - The backend does not report extra pages just because the chain or allocated length is larger.

### Scenario 2: Contiguous file page reads use the accepted placement boundary

- Test intent:
  - Confirm a contiguous regular-file backend read reaches the page level through `EXR-READ-11A` rather than a second mapping path.
- Suggested test shape:
  - Use a contiguous regular-file fixture and a block-device spy or stub that can prove the accepted mapping boundary was followed.
- Assertions:
  - The page read succeeds for an in-range page.
  - The page read uses the placement facts from `EXR-READ-11A`.
  - No fallback to buffered `read_at` is required to satisfy the page read.

### Scenario 3: FAT-backed file page reads use the accepted placement boundary

- Test intent:
  - Confirm a FAT-backed regular-file backend read still consumes the read-mapping boundary instead of re-deriving mapping locally.
- Suggested test shape:
  - Use a FAT-backed regular-file fixture with a chain that spans multiple clusters.
- Assertions:
  - The page read succeeds for an in-range page.
  - The backend follows the accepted placement facts.
  - The read path does not depend on a new FAT-walking helper in the page-cache component.

### Scenario 4: Pages at or beyond the backend-visible range stay zero-backed

- Test intent:
  - Confirm the backend-visible boundary leaves past-EOF pages to the page cache's zero-page behavior instead of inventing disk I/O.
- Suggested test shape:
  - Request a page index equal to or greater than the backend page count.
- Assertions:
  - The backend does not issue block-device I/O for the out-of-range page.
  - The page-cache path returns the expected zero/unbacked behavior.

### Scenario 5: Buffered read policy remains outside this component

- Test intent:
  - Confirm the backend contract is not secretly a buffered `read_at` implementation.
- Suggested test shape:
  - Keep the regression local to backend attachment and page-level I/O.
- Assertions:
  - No `read_at` copy policy is needed to satisfy the backend contract.
  - No write-side growth or truncation behavior is required for this component's regressions.

## Observability

- Use assertion macros only.
- Keep each test focused on one backend rule or boundary class.
- Prefer explicit fixtures over inline construction when the setup would obscure the visible-length rule under test.
- Do not inspect implementation-private fields except through the canonical backend surface needed to validate the contract.

## Acceptance Notes

- These regressions should validate the backend contract, not buffered read behavior.
- The checker does not need dedicated concurrency tests for this component.
- `02_designer_async.md` is not needed because the component has no separate publication, retry, or scheduling protocol beyond the existing page-cache backend contract.
