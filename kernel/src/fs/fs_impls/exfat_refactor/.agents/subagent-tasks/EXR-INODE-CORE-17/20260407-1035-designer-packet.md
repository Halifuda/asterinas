<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1035-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1035-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-INODE-CORE-17`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:35 CST`

## Goal

- Produce the designer artifact set for `EXR-INODE-CORE-17`, specifying the narrow `ExfatInode` VFS carrier and metadata owner from the accepted architect artifact without expanding into inode cache, page cache, directory ops, read/write, namespace mutation, or sync ordering.

## Architectural Unit Context

- Functional goal: introduce `ExfatInode` as the stable VFS `Inode` carrier with owner-private metadata and a weak `ExfatFs` back-reference.
- Final architectural owner: `ExfatInode`.
- Expected landing form: trait-carrier type plus owner state in a future `inode.rs`.
- Parent unit: none.
- Interfaces served: VFS `Inode` / `InodeIo`; sibling `EXR-FS-CORE-16` root handoff; future `EXR-INODE-CACHE-18`, `EXR-DIR-OPS-23`, `EXR-FILE-MAP-24`, `EXR-READ-OPS-25`, `EXR-PGCACHE-26`, `EXR-NAMESPACE-29`, and `EXR-WRITE-30`.

## Required Resolution Questions

- Point to the accepted architect artifact for unit boundary decisions.
- Specify the state fields and constructor inputs from trusted `ExfatDentrySet` and `ExfatChain` without retaining those validated value objects as surrogate owners.
- Specify the `Weak<ExfatFs>` back-reference and how `fs()` should behave in the initial carrier.
- Specify which metadata methods are meaningful now and which mutation/data-path methods remain explicit temporary seams or default rejections.
- Keep `InodeKey` and opened-inode table behavior out of this unit.
- Decide whether `02_designer_async.md` is needed. If omitted, record why no separate async/concurrency artifact is needed in `01_designer_core.md`.
- State checker-owned test obligations.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode_ext.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed when needed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/`
- Creator, checker, advisor, or reviewer artifacts.

## Required Inputs

- Role-scoped protocol files:
  - `COMMON_SUBAGENT.md`
  - `DESIGNER.md`
- Accepted architect artifact:
  - `components/EXR-INODE-CORE-17/00_architect.md`
- Sibling handshake artifact:
  - `components/EXR-FS-CORE-16/00_architect.md`

## Semantic Prior Inputs

- Use accepted `EXR-FILESET-04B` and `EXR-CHAIN-03B` behavior through `fileset.rs` and `fat.rs`.
- Do not read Linux source for this designer pass. Exact Linux inode hashing/writeback belongs to later `EXR-INODE-CACHE-18` and write-side units unless the main agent authorizes it.
- Precedence: accepted architect artifact, then accepted value-type code, then local VFS interfaces.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`, especially Inode contract, PageCache contract as future context, and current `ExfatInode` runtime-object context.
- Exact integration surfaces:
  - `fs_apis/inode.rs`
  - `fs_apis/inode_ext.rs`
  - `fs_apis/file_system.rs`

## Workflow Prior Inputs

- Command-free designer lane.
- May overlap with `EXR-FS-CORE-16` designer because write sets are disjoint.
- Creator-stage `mod.rs` declaration edits are a known shared-file collision and must be called out for scheduler serialization.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.
- Focus on canonical metadata surface, explicit temporary seam wording, no speculative helpers/accessors, and implementable creator pass split.

## Temporary Interfaces And Exit Plan

- Authorized temporary seams:
  - `InodeIo::read_at` and `InodeIo::write_at`, exiting to `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26`.
  - mutation methods such as `set_mode`, `set_owner`, or `set_group` only as explicit no-op/rejection seams if the designer requires them before write-side ownership exists.
- Do not specify cache, page-cache backend, directory lookup, namespace mutation, or writeback behavior here.

## Helper Justification

- Do not specify short helpers or field-exposing accessors unless the artifact names the caller or trust boundary. `InodeKey` helpers are forbidden in this unit.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-FS-CORE-16` designer lane.
- Known conflicts: future production `mod.rs` declaration edit may collide during creator work.

## Execution Environment

- Host or Docker: host read-only inspection.
- Required working directory: `/home/halifuda/asterinas`.
- This task must not add compile or runtime commands.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the assigned designer artifact set. If `02_designer_async.md` is omitted, explicitly say why in `01_designer_core.md`; otherwise write it.

## Escalation Rule

- If the accepted architect artifact is insufficient to specify inode metadata ownership without introducing cache or data-path behavior, stop and report the missing boundary rather than widening into later units.
