<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-04`
- Task packet: [`20260404-1454-architect-packet.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1454-architect-packet.md)

## Purpose

This handoff covers the smallest useful component that turns the accepted loaded upcase table from `EXR-UPCASE-07A` into a canonical case-folding and name-hash service for later filename work. It stays on the consumer side of the table boundary. It does not own table loading, mount bootstrap, root-directory discovery, lookup orchestration, or namespace mutation.

The component exists to replace the current provisional `fileset.rs` raw-UTF-16 `name_hash` behavior with a table-backed service that hashes already-folded UTF-16 units instead of raw code units.

## Why This Comes Now

`EXR-UPCASE-07A` already owns loading and validating the on-disk table bytes and exposes the canonical loaded-table surface. That makes the next split dependency-safe: the table is now a stable input, and the remaining work is pure name normalization on top of that value.

Microsoft exFAT and Linux both place upcase-table use after discovery and validation. Linux also splits `nls.c` from `namei.c`: the table-backed normalization belongs in the Unicode/name layer, while hashing and lookup policy are separate consumers. This component mirrors that split while keeping the surface narrower than Linux by excluding directory orchestration and mount policy.

## Dependency Contract

- Depends on:
  - `EXR-UPCASE-07A`
  - `EXR-FILESET-04B`
- Blocks:
  - `EXR-MOUNT-09`
  - `EXR-DIR-10`
  - `EXR-CREATE-12A`
  - `EXR-CREATE-12B`
  - `EXR-RENAME-13D`
- Can run in parallel with:
  - `EXR-BITMAP-08A` architect or designer work once `EXR-SYSROOT-06` is fixed
  - other command-free planning that does not need a mutable name-normalization surface
- Recommended parallel wave:
  - treat `EXR-UPCASE-07B` and `EXR-BITMAP-08A` as sibling post-`EXR-SYSROOT-06` consumers, with `EXR-UPCASE-07B` owning only case-fold and name-hash services layered on the accepted loaded table
- Stable pre-existing interfaces used:
  - the accepted loaded-table value from `EXR-UPCASE-07A`
  - the current `ExfatDentrySet` raw-name extraction path in `fileset.rs` as the place where the provisional raw-UTF-16 hash must be redirected
  - Microsoft exFAT `NameHash` rules
  - Linux `exfat_toupper()` and `exfat_d_hash()` as the algorithm and split reference
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for the table-backed upcase model and name-hash derivation on upcased UTF-16 bytes
  - `linux-exFAT-implementation-summary.md` plus Linux `fs/exfat/nls.c` and `fs/exfat/namei.c` for the normalization-versus-lookup split
  - `EXR-UPCASE-07A` artifacts for the canonical loaded-table boundary that this component must consume rather than recreate
  - `ASTERINAS_ARCHITECT_PRIORS.md` for the local rule that mount state, discovery, and later lookup orchestration must stay outside this component

## exFAT Concepts Covered

- Upcase-table-driven UTF-16 case folding.
- Name-hash derivation from folded UTF-16 units.
- Canonical consumption of the loaded upcase table produced by `EXR-UPCASE-07A`.
- Redirecting the provisional raw-UTF-16 `name_hash` path in `fileset.rs` to the table-backed service.
- Excluding lookup policy, trailing-dot policy, charset conversion, root discovery, fallback-table selection, and namespace mutation.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - None

## Code Budget

- Target new or heavily rewritten code size:
  - `160-240` lines
- Reason if the budget might exceed 500 lines:
  - It should not. If the slice grows beyond this, it is probably absorbing lookup comparison, trailing-dot policy, or mount-owned charset behavior that belong in later lookup or mount components.

## Exit Condition

Design work may start once there is exactly one canonical service surface that:

1. accepts the accepted loaded upcase table from `EXR-UPCASE-07A`,
2. folds UTF-16 input through that table,
3. derives the exFAT name hash from the folded UTF-16 units,
4. is used by `fileset.rs` instead of the current raw-UTF-16 hash helper,
5. does not load the table, rediscover the root entry, or make lookup or mount policy decisions.

## Risks

- The designer could leave the current raw-UTF-16 `name_hash` path in place and accidentally preserve the bug this component is meant to remove.
- The surface could drift into lookup comparison or trailing-dot trimming, which would pull policy back out of `EXR-DIR-10` and `EXR-MOUNT-09`.
- The component could grow a second helper for folding and hashing when one canonical service would be clearer and easier to consume.
- The design could reintroduce fallback-table selection or root discovery pressure from the legacy implementation.
- The table-backed service could be overfit to a specific caller and fail to remain a reusable boundary for later directory and rename work.
