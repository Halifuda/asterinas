<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-DESIGN-20260404-1501`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1501-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-UPCASE-07B`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:01 CST

## Goal

- Produce the bounded designer artifact set for `EXR-UPCASE-07B`: `01_designer_core.md` and `03_designer_ktest.md`. Add `02_designer_async.md` only if the component truly needs a distinct concurrency artifact.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/namei.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/02_designer_async.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `nls.c` and `namei.c` as needed for exact fold-and-hash behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - table-driven UTF-16 folding
  - exFAT `NameHash` derivation over folded UTF-16 bytes
  - replacing the provisional raw-UTF-16 hash path in `fileset.rs`

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
  - accepted `EXR-UPCASE-07A` artifacts
  - current `fileset.rs` call sites that still use `checksum_utf16(...)`
- Local focus:
  - one canonical read-only fold-and-hash service
  - no table loading, mount bootstrap, lookup policy, or namespace mutation
  - `fileset.rs` should consume the canonical service instead of keeping a second provisional hash implementation

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - one canonical interface surface
  - explicit invariants and failure cases
  - clear creator and checker split
- Out of scope:
  - creator-local naming or formatting trivia

## Prior Delivery Notes

- Make the minimum contract explicit: fold logical UTF-16 units through the loaded table, then derive the exFAT name hash from the folded units.
- If a separate fold-only helper is needed in addition to name hashing, justify why both surfaces are needed and which downstream caller uses each one.
- If no distinct concurrency story exists, omit `02_designer_async.md` instead of fabricating one.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper surface must name the downstream caller and explain why one canonical service is insufficient.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-MOUNT-09` architect work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-UPCASE-07B` designer artifacts

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free and must not add compile or runtime commands.

## Execution Lock

- Lock script:
  - not applicable
- Lock path:
  - not applicable
- Lock metadata file:
  - not applicable

## Stop Condition

- Stop after writing the required designer artifacts for `EXR-UPCASE-07B`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to the fold-and-hash service boundary, stop and report exactly what pressure is trying to pull mount, lookup, or loader ownership into this component.
