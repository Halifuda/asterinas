<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-READ-11A`
- Title: Logical-To-Physical Mapping For Existing Regular-File Reads
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11A-DESIGN-20260405-1059`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Checker-owned serial regressions for logical-to-physical placement on existing regular files.
  - Boundary coverage for contiguous placement, FAT-backed placement, and EOF handling.
  - Rejection coverage for directory shells and other non-regular-file inputs.
- Out of scope:
  - Async tests, background work, or concurrent publication protocols.
  - Page-cache behavior, buffered `read_at`, direct I/O, or zero-fill policy.
  - Allocation, truncation, or write-side mutation.

## Test Specification

- Test fixtures:
  - One contiguous regular-file fixture whose chain mode is `Contiguous`.
  - One FAT-backed regular-file fixture whose chain mode is `FatBacked`.
  - One non-regular-file fixture, preferably the explicit synthetic root shell or a directory shell built from validated facts.
  - One block-device stub that can prove the contiguous case does not perform FAT reads.
- Required checker-owned tests:
  - `contiguous_offset_maps_without_fat_reads`
    - Proves a logical offset maps to the expected physical cluster and byte offset when the chain is contiguous.
    - Proves the mapper does not touch the block device for a contiguous file.
  - `fat_backed_offset_maps_through_chain`
    - Proves a logical offset that spans multiple clusters resolves to the expected destination cluster and byte offset.
    - Proves the result uses the accepted chain facts rather than any arithmetic shortcut.
  - `offset_at_valid_data_end_returns_none`
    - Proves an offset equal to valid data length returns no placement.
    - Proves an offset beyond valid data length returns no placement as well.
  - `non_regular_file_is_rejected`
    - Proves a directory or root shell does not pass the read-mapping boundary.

## Observability

- Use assertion macros only.
- Keep each test focused on one boundary or behavior class.
- Prefer explicit fixture names over inline construction when the setup would obscure the placement rule under test.
- Do not inspect implementation-private fields except through the canonical test surface needed to validate the contract.

## Acceptance Notes

- The ktests should validate the placement boundary, not buffered copying or page-cache interaction.
- If a test needs to manufacture malformed chain state, that belongs to the chain component's coverage unless the read mapper adds a distinct boundary check.
- No dedicated concurrency tests are required for this component, and no `02_designer_async.md` file is needed.
