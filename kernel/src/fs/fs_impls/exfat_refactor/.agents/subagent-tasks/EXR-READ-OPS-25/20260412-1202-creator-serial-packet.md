<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-OPS-25-20260412-1202-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1202-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-READ-OPS-25`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 12:02 CST`

## Goal

- Implement the buffered regular-file `read_at` path on `ExfatInode` so `inode.rs` owns byte transfer, EOF truncation, short-read accounting, and valid-size zero-fill while consuming the accepted `EXR-FILE-MAP-24` translation helpers without widening into page cache, write-side mutation, or a filesystem-global reader.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered regular-file read path
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods and owner-private helpers in `inode.rs`
- Interfaces served:
  - VFS `InodeIo::read_at`
  - later inode-local cache population from `EXR-PGCACHE-26`
  - stable user-visible regular-file buffered read behavior

## Required Resolution Questions

- Replace the temporary `read_at` rejection in `inode.rs` with a real buffered regular-file read path.
- Preserve non-regular-file rejection instead of inventing directory or special-file reads.
- Consume `map_physical_file_range()` as a translation-only dependency.
- Source the current traversal context required by `EXR-FILE-MAP-24` through the inode owner boundary; if a thin `ExfatFs` helper or accessor is needed in `fs.rs`, keep it narrowly limited to block-device/super-block traversal context and record its removal or long-term owner condition in the creator artifact.
- Copy physically backed bytes into the caller-owned `VmWriter`, then zero-fill only the bounded `valid_size..size` gap.
- Return the total visible byte count and keep repeated-call behavior deterministic.
- Do not widen into page-cache ownership, write-side growth, truncate, allocator policy, or sync ordering.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/ostd/src/mm/io/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- all checker, reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-READ-OPS-25` designer set only; do not reopen architect or designer decisions unless the packet's escalation rule triggers.

## Semantic Prior Inputs

- `EXR-FILE-MAP-24` remains translation-only. Do not move EOF, short-read, or zero-fill policy back into mapping.
- Logical EOF is `self.size()`. Reads starting at or beyond logical EOF return `0`.
- The valid-size zero-fill region begins at `self.valid_size` and ends at logical EOF.
- The legacy exFAT inode source in the read set is reference material for byte-transfer shape only; it does not override the refactor's owner boundary or current helper constraints.

## Integration Prior Inputs

- The current `map_physical_file_range()` contract is accepted and includes temporary explicit traversal-context arguments. Consume that contract rather than redesigning mapping.
- If `fs.rs` changes are needed, they must stay as a very thin owner-boundary seam that only sources traversal context for the inode-owned read path. Do not create a reusable read service, generic device accessor layer, or page-cache hook.
- `write_at` remains a later seam. This row must not alter write-side ownership.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the only production creator lane in the current wave.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.
- Keep helper placement stable and local. Do not split fake parallel sub-lanes inside the same `inode.rs` read-path region.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep new read helpers owner-private to `ExfatInode`.
- Use checked arithmetic around offsets and lengths.
- If you introduce any temporary seam in `fs.rs`, record its exact purpose and intended absorption/removal condition in the creator artifact.

## Temporary Interfaces And Exit Plan

- A thin `ExfatFs` traversal-context helper or accessor is allowed only if it is the narrowest way to source the current `map_physical_file_range()` dependency contract.
- Do not add a public read helper, page-cache surface, write-side helper, or generic block-I/O service.
- Any remaining temporary seam must explicitly point forward to later absorption by the inode-owned buffered-read/cache path rather than becoming a new permanent owner.

## Helper Justification

- Allowed owner-private helpers may:
  - derive one bounded buffered-read iteration from the current logical offset,
  - copy one physically backed span into the caller-owned `VmWriter`,
  - and emit a bounded zero-filled tail for the valid-size gap.
- A thin `ExfatFs` helper or accessor is allowed only to surface traversal context already owned by the filesystem.
- Do not add helpers whose main effect is to invent a second reader owner or a generic data-path utility layer.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay in component artifacts
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - later checker or reviewer lanes for `EXR-READ-OPS-25`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`.
- Do not proceed into checker work.

## Escalation Rule

- If the accepted designer behavior still appears to require edits outside `inode.rs` and the allowed thin `fs.rs` traversal-context seam, or if implementation would require page-cache, write-side, or mapping-redesign decisions, report the exact missing handshake and stop instead of widening scope.
