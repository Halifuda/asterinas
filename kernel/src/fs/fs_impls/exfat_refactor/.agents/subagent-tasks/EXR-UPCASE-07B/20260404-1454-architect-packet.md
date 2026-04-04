<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-ARCH-20260404-1454`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1454-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-UPCASE-07B`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:54 CST

## Goal

- Write the architect artifact for `EXR-UPCASE-07B` in `00_architect.md`. The component must cover only case-fold and name-hash services layered on the accepted loaded upcase table from `EXR-UPCASE-07A`. It must not own table loading, mount bootstrap, directory lookup orchestration, or namespace mutation.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/namei.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted `EXR-UPCASE-07A` artifacts listed in the read set
- Required code/reference inputs:
  - current refactor `fileset.rs` and `upcase_table.rs`
  - legacy Asterinas upcase/name handling
  - Linux `nls.c` and `namei.c` as algorithm and split references

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `nls.c` and `namei.c` as needed for exact case-fold and name-hash behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - upcase-table-driven case folding
  - filename name-hash derivation on upcased UTF-16 units
  - separating validation or folding services from later directory lookup policy

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
  - accepted `EXR-UPCASE-07A` artifacts
  - the current `fileset.rs` provisional raw-UTF-16 `name_hash` behavior
- Local architectural focus:
  - `EXR-UPCASE-07A` already owns loading and validating the table bytes
  - `EXR-UPCASE-07B` should supply the canonical table-driven fold and hash boundary
  - `EXR-DIR-10` and `EXR-MOUNT-09` consume this service later but must not pull it backward into their own loaders

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - small dependency-safe split
  - preventing overlap with mount, directory, or loader ownership
  - naming one canonical service surface instead of several ad hoc helpers
- Out of scope:
  - creator-local implementation detail
  - final naming or formatting detail

## Prior Delivery Notes

- Keep this packet narrow around the second upcase component only: case folding and name-hash services built on the accepted loaded table.
- Explicitly address the current provisional `fileset.rs` raw-UTF-16 `name_hash` behavior and where the correction belongs.
- Do not widen into table loading, mount policy, or general lookup orchestration.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual canonical fold-and-hash surface.
- If the architect believes a short helper is needed, the artifact must name the downstream caller and explain why the surface cannot remain fully local to `EXR-UPCASE-07B`.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - any command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-UPCASE-07B` architect artifacts

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
- This task does not include a command-producing checker stage.

## Stop Condition

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the component cannot stay confined to case folding and name-hash services built on the loaded table, stop and report exactly what pressure is trying to pull table loading, mount policy, or directory lookup ownership into this component.
