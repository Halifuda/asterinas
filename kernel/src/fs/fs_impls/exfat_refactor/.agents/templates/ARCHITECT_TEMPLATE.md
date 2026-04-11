<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff Template

## Metadata

- Component ID:
- Title:
- Status: `Architected`
- Author:
- Date:
- Task packet:

## Functional Unit Definition

- Functional goal:
- Final architectural owner:
- Owner class:
  - VFS trait carrier,
  - structure owner,
  - daemon process,
  - or record type.
- Expected landing form:
- Boundary kind:
  - stable architectural boundary,
  - owner-internal slice,
  - owner-local structure,
  - daemon-process surface,
  - record-type boundary,
  - or temporary construction seam.
- Why this boundary is architecturally real:

## Purpose

Describe the smallest functionally coherent unit this handoff covers.

## Why This Comes Now

Explain why this component can be implemented at this stage without depending on later work.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
- Known non-goals or nearby logic that must remain in the parent owner:

## Dependency Contract

- Depends on:
- Blocks:
- Can run in parallel with:
- Recommended parallel wave:
- Stable pre-existing interfaces used:
- Prior sources or prior slices that materially shaped the split:

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the globally active plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-...` |  |  |  |  |  |  |  |

## exFAT Concepts Covered

List the exFAT structures, on-disk concepts, and VFS behaviors involved.

## Boundary Rejections

- Splits considered but rejected:
- Why those rejected splits would be packet convenience, not real architecture:

## Target Files

- Existing files likely to change:
- New files expected:

## Code Budget

- Target creator work-slice size:
- Expected number of creator slices:
- Reason if any single slice might exceed 500 lines:

## Exit Condition

State the observable condition that means design work may start.

## Risks

List details likely to be missed unless the designer treats them explicitly.
