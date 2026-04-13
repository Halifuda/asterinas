<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-CHARSET-32`
- Title: `ExfatFs` charset and external-name conversion boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1301-architect-packet.md`

## Functional Unit Definition

- Functional goal: give `ExfatFs` explicit ownership of VFS-visible exFAT name conversion, so later namespace and volume-label owners consume validated converted values instead of redoing charset work or reopening the upcase boundary, and so legacy read-side inode consumers stop performing ad hoc UTF-16 conversion in `inode.rs`.
- Final architectural owner: `ExfatFs`
- Owner class:
  - filesystem-wide service owner
- Expected landing form:
  - owner-internal conversion service plus validated converted-name/value types
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real:
  - Asterinas VFS already exposes names as Rust `&str`, so the external contract is UTF-8 text, not Linux's byte-string charset API.
  - exFAT still needs a filesystem-owned conversion step that validates the external string shape and produces canonical UTF-16 units for later use.
  - That conversion is filesystem-wide policy, not inode-local namespace logic and not volume-label mutation logic.

## Purpose

This unit is the smallest coherent slice that gives `ExfatFs` the name/label codec boundary without reopening `EXR-UPCASE-20`.
`EXR-UPCASE-20` still owns fold/hash over UTF-16 units through the validated upcase table.
`EXR-CHARSET-32` sits before that boundary and normalizes the VFS-visible external string into a validated converted value that later rows can consume safely.
It also owns the inverse visible-name decode needed by legacy read-side consumers so `inode.rs` does not keep local `encode_utf16()` or `String::from_utf16()` policy after this row lands.

The owner boundary should answer five questions only:

1. what external string shape exFAT accepts from Asterinas VFS,
2. how that string becomes validated UTF-16 units,
3. how validated UTF-16 name units become visible VFS strings again on read-side paths,
4. how later rows receive the converted value without redoing charset work,
5. how the same conversion service can be reused by name, readdir, and volume-label consumers.

Everything else stays elsewhere.

## Why This Comes Now

The board already has the forward-path consumers that need a real charset boundary:

- `EXR-NAMESPACE-29` must canonicalize user-visible child names before it can ask `EXR-UPCASE-20` for fold/hash behavior.
- `EXR-VOLLABEL-35` needs the same external-string-to-UTF-16 conversion boundary for volume-label strings, but it does not need namespace mutation or upcase folding.

It also already has one accepted read-side consumer that still carries legacy local conversion:

- `EXR-DIR-OPS-23` currently implements `lookup` and `readdir_at` in `inode.rs`, and the current code still performs local `encode_utf16()` for lookup plus local `String::from_utf16()` for visible-name decode.

Without this row, those consumers would either inline UTF-8/UTF-16 conversion in `inode.rs` and `fs.rs`, or they would smuggle in a generic text helper module.
Both outcomes would be architecture drift.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - existing `EXR-DIR-OPS-23` read-side lookup preflight and visible-name projection as legacy consumer repairs
  - future `EXR-NAMESPACE-29` namespace mutation preflight
  - future `EXR-VOLLABEL-35` volume-label control
  - any later VFS-visible exFAT path that accepts a Rust `&str` and must become canonical UTF-16, or that must project validated UTF-16 back to a visible Rust string
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is internal to `ExfatFs`, but the boundary is stable because every later VFS-visible name/label consumer must cross the same codec step before it can talk to on-disk exFAT records.
- Known non-goals or nearby logic that must remain in the parent owner:
  - upcase-table installation
  - UTF-16 case folding
  - exFAT name hashing
  - namespace mutation
  - volume-label mutation
  - directory scanning
  - sync ordering
  - allocation search or reservation

Boundary consumption rules:

- `EXR-DIR-OPS-23` should stop doing local `encode_utf16()` or `String::from_utf16()` in `inode.rs`; its lookup and readdir code paths should consume this row as a legacy repair without reopening read-side ownership.
- `EXR-NAMESPACE-29` should consume a validated converted-name value from this row, then pass its UTF-16 units to `EXR-UPCASE-20` for fold/hash work.
- `EXR-VOLLABEL-35` should consume the same external-string conversion boundary for label strings, but only as a validated converted-label value; it should not consume fold/hash behavior.
- Neither row should receive raw `&str` parsing helpers, a locale-sensitive NLS loader, or a generic Unicode helper module.
- `EXR-UPCASE-20` remains the only owner of canonicalization after UTF-16 conversion.

## Dependency Contract

- Depends on:
  - `EXR-UPCASE-20` for UTF-16 fold/hash behavior on already-converted units
  - `EXR-FS-OPEN-22` for mount/open context and the already-published `ExfatFs` owner
  - the Asterinas VFS string contract, which uses Rust `&str` values
  - the exFAT name and label semantics already recorded in the Microsoft spec and the authorized Linux source references
- Blocks:
  - legacy `lookup` / `readdir_at` consumer cleanup under `EXR-DIR-OPS-23`
  - later `EXR-NAMESPACE-29` name preflight
  - later `EXR-VOLLABEL-35` label control
- Can run in parallel with:
  - owner work that does not touch `fs.rs`
  - later inode-side work only once it consumes a validated converted value rather than redoing codec work
- Recommended parallel wave:
  - Wave C after `EXR-UPCASE-20` and `EXR-FS-OPEN-22` are already specified, because this row depends on the installed filesystem owner and on the canonicalization service that will consume its converted values
- Stable pre-existing interfaces used:
  - `ExfatFs`
  - `ExfatInode`
  - `ExfatDentrySet`
  - `EXR-UPCASE-20` owner methods
  - the VFS `Inode` name-bearing methods
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-POST28/00_architect.md`
  - `EXR-UPCASE-20/00_architect.md`
  - `EXR-NAMESPACE-29/00_architect.md`
  - `EXR-DIR-OPS-23/01_designer_core.md`
  - `linux-exFAT-implementation-summary.md`
  - `fs/exfat/exfat_fs.h`
  - `fs/exfat/nls.c`
  - `fs/exfat/namei.c`

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-CHARSET-32-CONVERT` | `EXR-CHARSET-32` | Define the validated external-string conversion boundary inside `ExfatFs`, accepting `&str` names and labels and producing owner-private validated UTF-16 values. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-FS-OPEN-22` | `EXR-UPCASE-20` architect/design only; not file-parallel with sibling lanes that also need `fs.rs` | creator | Keep this slice focused on validation and UTF-16 conversion. Do not fold, hash, or mutate directory records here. |
| `WS-CHARSET-32-CONSUME` | `EXR-CHARSET-32` | Add owner methods or owner-private helpers that hand validated converted values to later namespace and volume-label callers, and visible-name decode results to legacy read-side callers, without exposing raw codec state. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `WS-CHARSET-32-CONVERT`, `EXR-UPCASE-20` | `EXR-NAMESPACE-29`, `EXR-VOLLABEL-35`, and accepted `EXR-DIR-OPS-23` consumer code paths; not file-parallel if they still want the same owner-file regions | creator | This slice should stay a service surface, not a generic text helper module. The converted value type and visible-name decode helper are real boundaries; legacy read-side consumption belongs here rather than inlining policy in `inode.rs`. |

## exFAT Concepts Covered

- UTF-8 external names from Asterinas VFS.
- UTF-16 conversion for exFAT name and label payloads.
- UTF-16-to-visible-string decode for read-side name projection.
- Validation of canonical converted values before later consumers use them.
- Separation of external-string conversion from upcase-table fold/hash services.
- Reuse of one filesystem-owned codec boundary by lookup, readdir, namespace, and volume-label control.

## Boundary Rejections

- Splitting charset work into a free helper module was rejected. That would be packet convenience, not a stable filesystem owner boundary.
- Treating this row as Linux's optional byte-string NLS layer was rejected. Asterinas already presents names as Rust `&str`, so the stable external contract is UTF-8 text.
- Folding case or computing name hashes here was rejected. `EXR-UPCASE-20` already owns that boundary.
- Pulling namespace mutation into this row was rejected. `EXR-NAMESPACE-29` should consume the converted value, not own codec policy.
- Pulling volume-label mutation into this row was rejected. `EXR-VOLLABEL-35` should consume the converted label value, not own codec policy.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` as a consumer migration point for accepted read-side directory ops and later namespace work
- New files expected:
  - none required by this boundary
- Future collision risk to watch:
  - `fs.rs` if this row lands near `EXR-FS-OPEN-22`, `EXR-SYNC-31`, or `EXR-UPCASE-20`
  - `inode.rs` for both legacy read-side consumer repair and later namespace consumption, but still not as an owner of codec policy

## Code Budget

- Target creator work-slice size: `180-280` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines:
  - it should not. If it does, the slice has probably absorbed namespace mutation, label mutation, or fold/hash behavior, which means the boundary has drifted.

## Exit Condition

Design work may start once `ExfatFs` clearly owns a UTF-8 external-name conversion boundary that can produce validated UTF-16 values for later namespace and volume-label consumers, while `EXR-UPCASE-20` remains the only owner of fold/hash behavior over those converted units.

## Risks

- The conversion path can accidentally become a generic Unicode helper if later callers are not named and constrained.
- The accepted read-side lookup/readdir row can accidentally retain ownerless codec policy if `lookup` keeps `encode_utf16()` and `readdir_at` keeps `String::from_utf16()` in `inode.rs`.
- The namespace consumer can accidentally smuggle fold/hash logic back into `inode.rs` unless it receives only the validated converted-name value.
- The volume-label consumer can accidentally reopen charset policy if it starts asking for a separate NLS layer instead of reusing the same external-string conversion boundary.
- If the design starts treating Linux's optional byte-string policy as the stable contract, the row has widened beyond the Asterinas VFS surface.
