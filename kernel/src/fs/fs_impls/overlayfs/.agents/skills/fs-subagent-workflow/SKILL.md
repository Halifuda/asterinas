---
name: fs-subagent-workflow
description: Role workflow for delegated Designer, Creator, Checker, and Reviewer work in `kernel/src/fs/fs_impls/overlayfs`. Use when a dispatch stub assigns one bounded non-Architect filesystem implementation/refactor step and you need the packet boundary, role rules, and checker execution constraints without loading the full scheduler protocol.
---

# Filesystem Subagent Workflow

Use this skill for ordinary delegated work inside the `overlayfs` protocol.
If the packet is an Architect packet or the work is about `macro_00_global_topology` or meso architecture mapping, switch to `$fs-architect-agent`.

## Quick start

1. Open the archived packet first.
2. Open `references/common-subagent.md`.
3. Open the matching role note:
   - `references/designer.md`
   - `references/creator.md`
   - `references/checker.md`
   - `references/reviewer.md`
4. Open the repository role source when needed:
   - `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/DESIGNER.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/CREATOR.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/CHECKER.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/REVIEWER.md`
5. If the packet allows checker execution or validation-harness work, also open `references/testing-guide.md`.
6. If the packet permits scoped Asterinas code inspection, prefer the `ra-code-nav` skill (LSIF index + `jq`) for rust-analyzer symbol navigation before broad `rg` / file search. It does not authorize reading outside packet scope.
7. Stay inside the packet boundary. If required input is missing, stop and report the gap instead of widening scope.
8. For structural-cleanup packets, do not assume only newly introduced entities are in scope. If the packet names surviving helpers, thin endian wrappers, or legacy test-only support layout as review/cleanup targets, audit that full surface directly.

## Core rule

Packet-specific instructions override this skill when they are more specific.
This skill is the reusable default.
The packet is the per-task contract.

Structural defaults to remember when the packet points at code-quality cleanup:
- treat clusters of naked helper functions as one structural surface rather than isolated symbols
- prefer direct fixed-width `from_le_bytes(...)` decoding over duplicated thin `read_le_*` wrappers unless a real local semantic contract exists
- do not add or recommend filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`; new validation must use the upstream-approved external/system-level lane

## Reference map

- `references/common-subagent.md`
  Scope, packet authority, and command discipline shared by all ordinary delegated roles.
- `references/designer.md`
  How to produce the unified Designer spec plus Designer validation contract.
- `references/creator.md`
  How to implement a bounded pass or a Checker-produced repair batch without redesigning.
- `references/checker.md`
  How to validate one pass through the upstream-approved lane, inspect preserved guest logs and suite results, and issue repair batches.
- `references/reviewer.md`
  How to do bounded static quality review after the implementation and checker loops settle.
- `references/testing-guide.md`
  Shared guidance for the Checker execution lane and upstream-approved validation proof.
