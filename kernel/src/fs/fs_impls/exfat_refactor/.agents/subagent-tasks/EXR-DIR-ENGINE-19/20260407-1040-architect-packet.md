<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-ENGINE-19-20260407-1040-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1040-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-DIR-ENGINE-19`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:40 CST`

## Goal

- Produce the architect artifact for `EXR-DIR-ENGINE-19`: the owner-first boundary for an `ExfatFs`-owned internal `DirectoryEngine` that streams directory contents as `ExfatDentrySet` records over an `ExfatChain`, without absorbing upcase/name policy, bitmap loading, VFS directory ops, or write-side directory mutation.

## Architectural Unit Context

- Functional goal: define the stable internal directory record-stream service used by mount-time system-entry discovery and later `ExfatInode` directory operations.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal service `DirectoryEngine`.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces served: future `EXR-UPCASE-20`, `EXR-BITMAP-21`, `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and later `EXR-DENTRY-WRITE-28`.

## Required Resolution Questions

- What is the read-only directory record-stream boundary, and what should remain outside it?
- How should the engine consume `read_metadata_bytes`, `ExfatChain`, and `ExfatDentrySet` without becoming free helper functions?
- What work slices are safe before upcase/bitmap/name policy exists?
- Which later write-side methods are explicitly out of scope and must wait for `EXR-DENTRY-WRITE-28`?
- What file landing zones and future collision risks should the designer know?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Other role artifacts outside the write set.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `ARCHITECT.md`.
- Board reset artifact: `WORKSPACE-ARCH-RESET/00_architect.md`.
- Accepted foundation code: `io.rs`, `fat.rs`, `dentry.rs`, `fileset.rs`.

## Semantic Prior Inputs

- Use `linux-exFAT-implementation-summary.md` topic “Directory record parsing and dentry-set validation” for orientation only.
- No direct Linux source reads are authorized. If exact Linux directory scanning state machine behavior is needed, stop and report.
- Use accepted `EXR-IO-02`, `EXR-CHAIN-03B`, and `EXR-FILESET-04B` behavior as primary local inputs.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially mount/open sequence and directory/runtime owner context.
- Legacy exFAT inode directory code is integration context only.

## Workflow Prior Inputs

- Command-free architect lane.
- May overlap with `EXR-INODE-CACHE-18` architect and Wave A designer lanes because write sets are disjoint.
- Workflow priors may shape work slices only after owner and read-only engine boundary are resolved.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- Focus on keeping the engine a stable `ExfatFs` internal service, not a pile of free helper functions.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner/removal condition.

## Helper Justification

- Any helper-like surface must be justified by `DirectoryEngine` ownership and future consumers, not by packet convenience.

## Allowed Commands

- Read-only shell commands only.
- No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-INODE-CACHE-18` architect and Wave A designer lanes.
- Known conflicts: none beyond forbidden write sets.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `components/EXR-DIR-ENGINE-19/00_architect.md`.

## Escalation Rule

- If the read-only directory engine boundary cannot be defined without upcase/bitmap or VFS directory ops, stop and report the missing dependency instead of widening the component.
