<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-SYSROOT-06`
- Title: Root-directory system-entry scanner
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-04`
- Task packet: [`EXR-SYSROOT-06-ARCH-20260404-1402`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYSROOT-06/20260404-1402-architect-packet.md)

## Purpose

This handoff covers the smallest useful exFAT component for scanning the root directory for validated system entries and exposing only the discovery facts that later loaders need. It follows the task packet `EXR-SYSROOT-06-ARCH-20260404-1402` and keeps the component strictly out of mount bootstrap, general directory iteration, inode/VFS behavior, and the concrete `UPCASE` / `BITMAP` loaders.

The expected product is a narrow read-only scanner result that says whether the root directory contains the required system entries, where each accepted entry lives on disk, and which entry-local validation facts were proven at discovery time. It does not load bitmap bytes, does not load upcase bytes, and does not own any shared mount state.

## Why This Comes Now

The downstream `UPCASE-07A` and `BITMAP-08A` work depends on a narrow, dependency-safe discovery step: identify which root-directory system entries are present, validate them at the boundary, and make their locations and basic identities available for later dedicated loaders.

That split is now safe because:

- `EXR-IO-02` already provides read-side metadata I/O,
- `EXR-CHAIN-03B` already provides validated root-chain traversal,
- `EXR-DENTRY-04A` already provides typed single-dentry decoding,
- `EXR-INODE-05B` already provides the synthetic root metadata shell that can own the root chain facts without inventing a mount object.

Legacy Asterinas and Linux both currently interleave root scanning with concrete upcase and bitmap loading. This component exists to break that coupling before `EXR-UPCASE-07A` and `EXR-BITMAP-08A` are implemented.

## Dependency Contract

- Depends on:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-INODE-05B`
- Blocks:
  - `EXR-UPCASE-07A`, because it needs a validated root-directory `UPCASE` entry identity and location.
  - `EXR-BITMAP-08A`, because it needs a validated root-directory allocation bitmap entry identity and location.
  - no other component should rediscover these root system entries independently.
- Can run in parallel with:
  - command-free planning work for the later `UPCASE-07A` / `BITMAP-08A` loaders.
- Recommended parallel wave:
  - architect `EXR-SYSROOT-06` first;
  - once its boundary is accepted, immediately overlap `EXR-SYSROOT-06` design and implementation with architect/design preparation for `EXR-UPCASE-07A` and `EXR-BITMAP-08A`.
- Stable pre-existing interfaces used:
  - `ExfatSuperBlock::root_dir` and cluster-size / cluster-to-offset helpers from `super_block.rs`
  - `ExfatChain` root traversal from `fat.rs`
  - typed raw-dentry decoding from `dentry.rs`
  - the synthetic root metadata shell from `inode.rs`
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for root-directory system-entry semantics and the on-disk identity of allocation bitmap and upcase-table entries.
  - `linux-exFAT-implementation-summary.md`, plus Linux `super.c`, `balloc.c`, and `nls.c`, for the current discovery ordering and the split pressure between scanning and later content loading.
  - `ASTERINAS_ARCHITECT_PRIORS.md` for the local rule that mount/bootstrap, bitmap management, and upcase loading must become separate components.
  - legacy Asterinas `exfat/fs.rs`, `bitmap.rs`, and `upcase_table.rs` as integration-pressure references only.

## exFAT Concepts Covered

- Root-directory system-entry scanning.
- Validation of root-directory entries before later loaders act on them.
- Discovery of the allocation bitmap entry and upcase-table entry as root-directory facts only.
- The scanner result should preserve only facts such as:
  - entry kind,
  - validated start cluster,
  - validated byte size,
  - checksum field when the later loader needs it,
  - and the primary-dentry location used to derive stable ownership or diagnostics later.
- Boundary checks needed to keep later `UPCASE` and `BITMAP` loaders from re-validating or re-discovering the same ownership questions.
- Duplicate-entry, missing-entry, wrong-kind, or obviously malformed-entry detection at the root boundary.
- No mount object semantics.
- No general directory walking API.
- No early loading of bitmap contents.
- No early loading of upcase contents.
- No inode/VFS behavior.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - Roughly `220-300` lines when implemented as a focused scanner and result type.
- Reason if the budget might exceed 500 lines:
  - If the scanner is forced to absorb mount ownership, directory traversal, or concrete loader state, that would signal a scope violation rather than a justified growth in this component. The packet does not authorize that expansion.

## Exit Condition

Design may start when the implementation plan expresses the root-directory scanner as a narrow discovery-and-validation unit whose only outputs are the validated system-entry facts required by later loaders, with:

1. one canonical scanner entry point for the root directory,
2. one read-only result type that can represent the accepted system entries without becoming shared mount state,
3. explicit ownership of duplicate/missing/malformed system-entry detection,
4. no mount bootstrap, no general directory API, and no bitmap or upcase content loading.

## Risks

- The scanner could accidentally become a mount-owned bootstrap object if it starts storing shared filesystem state instead of returning narrow discovery results.
- The scanner could drift into general directory iteration if it is used to enumerate arbitrary entries rather than only the root-directory system entries relevant to later loaders.
- The scanner could be overextended into `UPCASE` or `BITMAP` loading if it begins reading or caching the table contents rather than only validating and identifying the entries.
- The scanner could become a hidden dependency sink if it exposes convenience helpers that belong in later `UPCASE`, `BITMAP`, or directory work.
- If validation responsibilities are not sharply bounded here, downstream loaders may duplicate checks or, worse, rely on incomplete discovery data.
- The root chain must be treated as already-validated input from accepted read-side components; if the scanner starts rebuilding root identity or mount ownership, it has crossed back into `EXR-INODE-05B` or forward into `EXR-MOUNT-09`.
