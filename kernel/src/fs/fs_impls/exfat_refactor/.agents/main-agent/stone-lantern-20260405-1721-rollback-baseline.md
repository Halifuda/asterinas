<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `stone-lantern`
- Date: 2026-04-05 17:21 CST
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Rollback checkpoint; no active implementation wave

## Rollback Summary

- The refactor is intentionally rolled back to the post-`EXR-SBGEOM-15`, pre-`EXR-INOKEY-05A` baseline.
- Preserved current protocol/process content: `README.md`, `PROTOCOL.md`, `.agents/protocol/`, `.agents/templates/`, `TESTING_GUIDE.md`, priors, project brief, and checker tooling.
- Removed later code, packets, component artifacts, and main-agent handoffs from the failed wave beginning with `EXR-INOKEY-05A`.

## Why This Rollback Happened

- We now treat the architect flow from `INOKEY` onward as failed.
- The slices stopped converging on concrete VFS trait carriers and instead produced ownerless staging surfaces, free helpers, and intermediate structs that did not materially close the gap to `ExfatFs` / `ExfatInode` trait implementations.
- The rollback preserves the workflow and protocol refinements, but discards the code/task history produced under that drift.

## Current Project State

- Accepted code/artifacts stop at:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
  - `EXR-SBGEOM-15`
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
- `COMPONENT_INDEX.md` is reset to the corresponding pre-`INOKEY` planning state.
- No creator, checker, reviewer, or resume loop should start until a replacement architect rubric is defined.

## Next Main-Agent Tasks

1. Read `README.md`, `PROTOCOL.md`, `COMPONENT_INDEX.md`, and this handoff.
2. Treat the current tree as a rollback baseline, not as an in-progress wave to resume.
3. Do not launch new component work until the architect stage is redefined around trait carriers and owner-first composition.
