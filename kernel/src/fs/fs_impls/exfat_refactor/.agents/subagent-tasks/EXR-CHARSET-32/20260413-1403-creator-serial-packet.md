<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1403-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1403-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-CHARSET-32`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 14:03 CST`

## Goal

- Implement the `ExfatFs`-owned charset boundary for visible names and labels so `fs.rs` owns validated external-name conversion plus visible-name decode, while `inode.rs` stops carrying local `encode_utf16()` and `String::from_utf16()` policy in accepted read-side directory ops.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned charset and visible-name conversion boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private converted-name / converted-label value types plus owner methods in `fs.rs`, with narrow consumer migration in `inode.rs`
- Interfaces served:
  - current `ExfatInode::lookup` read-side name preflight
  - current `ExfatInode::readdir_at` visible-name projection
  - later `EXR-NAMESPACE-29` namespace mutation preflight
  - later `EXR-VOLLABEL-35` volume-label control

## Required Resolution Questions

- Add the validated `ConvertedName` and `ConvertedLabel` boundary under `ExfatFs` in `fs.rs`.
- Implement `&str` -> validated UTF-16 conversion for names and labels without attaching fold/hash state.
- Implement the inverse visible-name decode path for validated on-disk UTF-16 units so `inode.rs` does not choose decode policy locally.
- Migrate accepted read-side directory ops in `inode.rs` so `lookup` no longer calls `encode_utf16()` directly and `readdir_at` no longer calls `String::from_utf16()` directly.
- Keep fold/hash ownership under the existing `EXR-UPCASE-20` owner methods already in `fs.rs`.
- Keep low-level UTF-16 leaf seams such as `ExfatDentrySet::from_trusted_metadata(..., raw_name_units, ...)` untouched unless the escalation rule triggers.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-CHARSET-32` designer set plus the accepted `EXR-DIR-OPS-23` consumer surface only; do not reopen architect or designer decisions unless the escalation rule triggers.

## Semantic Prior Inputs

- Asterinas VFS still presents names as `&str`; do not change VFS signatures.
- `EXR-CHARSET-32` owns visible-name encode and decode policy for exFAT.
- `EXR-UPCASE-20` remains the only owner of UTF-16 fold/hash behavior; converted values must not attach hash state here.
- `lookup` must still perform case-insensitive matching through the installed upcase table, but it should first cross the `ExfatFs` charset boundary rather than building UTF-16 locally.
- `readdir_at` must still emit VFS-visible names, but decode policy must cross the `ExfatFs` charset boundary rather than `String::from_utf16()` inside `inode.rs`.
- Low-level trusted constructors that accept raw UTF-16 units may remain as leaf seams; business-facing name paths must stop feeding them ad hoc UTF-16 built outside `ExfatFs`.

## Integration Prior Inputs

- `EXR-DIR-OPS-23` remains accepted and should not be reopened as an ownership change; this pass is only a consumer migration inside the already-accepted read-side path.
- `EXR-NAMESPACE-29` is specified against the converted-name handoff, so helper shape in `fs.rs` should remain reusable by later namespace work.
- Keep this pass out of namespace mutation, volume-label mutation, directory-entry writes, and sync ordering.
- Avoid widening `fs.rs` into a generic Unicode helper surface or a second text subsystem.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the current wave's active production creator lane.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.
- Record any new owner-private helper, temporary seam, or local type in the creator artifact with its final owner or removal condition.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer owner methods, owner-private helpers, and owner-private value types on `ExfatFs` over module-scope convenience functions.
- Keep the `inode.rs` migration narrow: consume the charset owner there, do not invent a lookup service or read-side helper module.
- Use checked length handling around UTF-16 conversion and visible-name decode.

## Temporary Interfaces And Exit Plan

- New owner-private value types and helper methods in `fs.rs` are allowed if they remain subordinate to `ExfatFs`.
- A small `inode.rs` helper is allowed only if it keeps lookup/readdir consumption readable and clearly subordinate to `ExfatInode`.
- Do not add a generic `Utf16Name` utility module, public accessor surface, or helper namespace that floats outside `ExfatFs`.
- Do not alter low-level file-record constructors in this pass; later rows may continue to consume them as trusted leaf seams after crossing the charset boundary.

## Helper Justification

- Allowed helpers may:
  - validate and materialize one converted-name or converted-label value under `ExfatFs`,
  - decode one validated UTF-16 record-name into a visible `String`,
  - and let `inode.rs` consume those owner methods without duplicating conversion policy.
- Reject helpers whose main effect is to invent a generic text subsystem or to move fold/hash ownership away from the existing upcase owner methods.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay outside `fs.rs` and `inode.rs`
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - later checker or reviewer lanes for `EXR-CHARSET-32`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`.
- Do not proceed into checker work.

## Escalation Rule

- If implementation appears to require edits outside `fs.rs`, `inode.rs`, and the creator artifact, or if it appears to require reopening `EXR-DIR-OPS-23`, `EXR-UPCASE-20`, or low-level file-record constructors, report the exact missing handshake and stop instead of widening scope.
