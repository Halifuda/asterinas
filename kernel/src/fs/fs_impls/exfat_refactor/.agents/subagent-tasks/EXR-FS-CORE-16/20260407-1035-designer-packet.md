<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-CORE-16-20260407-1035-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1035-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-FS-CORE-16`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:35 CST`

## Goal

- Produce the designer artifact set for `EXR-FS-CORE-16`, specifying the narrow `ExfatFs` owner skeleton from the accepted architect artifact without expanding into mount/open, inode cache, directory services, allocation, or sync ordering.

## Architectural Unit Context

- Functional goal: introduce `ExfatFs` as the stable VFS `FileSystem` carrier and filesystem-wide runtime-state root.
- Final architectural owner: `ExfatFs`.
- Expected landing form: trait-carrier type plus owner state in a future `fs.rs`.
- Parent unit: none.
- Interfaces served: VFS `FileSystem`; future `EXR-FS-OPEN-22`; sibling `EXR-INODE-CORE-17`; future `EXR-INODE-CACHE-18`, `EXR-DIR-ENGINE-19`, `EXR-SYNC-31`.

## Required Resolution Questions

- Point to the accepted architect artifact for unit boundary decisions.
- Specify exactly which `FileSystem` methods are in scope now: accepted architect says `name()`, `sb()`, and `fs_event_subscriber_stats()` land now; `root_inode()` is an explicit temporary seam; `sync()` must not pull in real flush ordering.
- Specify the temporary `root_inode()` seam, including the required comment and exit plan into `EXR-FS-OPEN-22`.
- Decide whether `02_designer_async.md` is needed. If omitted, record why no separate async/concurrency artifact is needed in `01_designer_core.md`.
- State test obligations for checker-owned coverage of the skeleton and temporary seam.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed when needed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/`
- Creator, checker, advisor, or reviewer artifacts.

## Required Inputs

- Role-scoped protocol files:
  - `COMMON_SUBAGENT.md`
  - `DESIGNER.md`
- Accepted architect artifact:
  - `components/EXR-FS-CORE-16/00_architect.md`
- Sibling handshake artifact:
  - `components/EXR-INODE-CORE-17/00_architect.md`

## Semantic Prior Inputs

- Use accepted boot/superblock facts from `boot_sector.rs` and `super_block.rs`.
- Do not read Linux source for this designer pass. The accepted architect artifact already bounds Linux material to high-level orientation.
- Precedence: accepted architect artifact, then local VFS interfaces, then accepted boot/superblock code.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`, especially `FileSystem` contract and current `ExfatFs` runtime-object context.
- Exact integration surfaces:
  - `fs_apis/file_system.rs`
  - `fs_apis/inode.rs`

## Workflow Prior Inputs

- Command-free designer lane.
- May overlap with `EXR-INODE-CORE-17` designer because write sets are disjoint.
- Creator-stage `mod.rs` declaration edits are a known shared-file collision and must be called out for scheduler serialization.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.
- Focus on canonical interface surface, invariant wording, temporary seam comment/exit plan, and implementable creator pass split.

## Temporary Interfaces And Exit Plan

- Authorized temporary seam: `FileSystem::root_inode()`, with the architect-approved comment:
  - `// Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.`
- Real root construction must remain out of scope.

## Helper Justification

- Do not specify short helpers or field-exposing accessors unless the artifact names the caller or trust boundary. The default expectation is no helper APIs beyond the owner skeleton.

## Allowed Commands

- Read-only shell commands only.
- No build, test, format, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-INODE-CORE-17` designer lane.
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

- If the accepted architect artifact is insufficient to specify the `root_inode()` seam or sync placeholder, stop and report the missing boundary rather than widening into `EXR-FS-OPEN-22` or `EXR-SYNC-31`.
