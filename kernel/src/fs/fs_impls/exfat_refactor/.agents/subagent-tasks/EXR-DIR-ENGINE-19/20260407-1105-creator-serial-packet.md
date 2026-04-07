<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-ENGINE-19-20260407-1105-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1105-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-DIR-ENGINE-19`
- Phase: `serial implementation`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:05 CST`

## Goal

- Implement the read-only `DirectoryEngine` record-stream service from the accepted designer spec without absorbing name policy, bitmap policy, VFS directory operations, or write-side mutation.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned internal directory record stream.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal `DirectoryEngine` service in `directory.rs`.
- Parent units: `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`.
- Interfaces served: future `EXR-UPCASE-20`, `EXR-BITMAP-21`, `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and `EXR-DENTRY-WRITE-28`.

## Required Resolution Questions

- Add `DirectoryEngine` as an owner-internal read-only service with validated `ExfatChain` traversal input and private scan cursor state.
- Emit validated `ExfatDentrySet` values for full file-record shapes while preserving on-disk order.
- Surface singleton `Bitmap` and `Upcase` candidates as raw typed dentries only.
- Skip `Deleted` entries and treat `Unused` as the end-of-directory marker.
- Keep name folding, name hashing, bitmap loading, upcase loading, VFS directory ops, and write-side mutation out of this pass.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CREATOR.md`.
- Designer spec: `EXR-DIR-ENGINE-19/01_designer_core.md` and `03_designer_ktest.md`.
- Creator log template.

## Semantic Prior Inputs

- Use accepted designer constraints and accepted refactor value-type boundaries only. Do not reopen Linux behavior.

## Integration Prior Inputs

- Use `read_metadata_bytes`, `ExfatChain`, raw/typed dentry boundaries, `ExfatDentrySet`, and `ExfatSuperBlock` as already accepted by prior components.

## Workflow Prior Inputs

- Command-free creator lane. Do not run compile, test, format, Docker, KVM, or QEMU commands.
- Launch only after `EXR-FS-CORE-16` reviewer is complete if the `mod.rs` declaration remains a live collision point.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CREATE`.
- Keep visibility narrow and avoid helper wrappers without designer-backed justification.

## Temporary Interfaces And Exit Plan

- No long-lived temporary staging surface is authorized.
- If a small private scan-state representation is needed, keep it inside `DirectoryEngine` and document any later consumer only in the creator log.

## Helper Justification

- Helper methods are justified only when they keep record-stream parsing local to `DirectoryEngine`.
- Do not expose general-purpose name, bitmap, upcase, or mutation helpers.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, Docker, KVM, or QEMU commands.

## Parallelism Classification

- Lane class: command-free production edit.
- Known conflicts: owns `directory.rs` and may edit `mod.rs`; do not launch while another active lane is editing `mod.rs`.
- May overlap with `EXR-INODE-CACHE-18` creator only if the sibling lane is restricted to `fs.rs` and no shared file edits are active.

## Execution Environment

- Host read-only inspection and file edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after implementing the assigned pass and writing `EXR-DIR-ENGINE-19/10_creator_serial.md`.

## Escalation Rule

- If the directory engine cannot be implemented without name policy, bitmap policy, write-side mutation, or edits outside `directory.rs`/`mod.rs`, report the gap instead of widening scope.
