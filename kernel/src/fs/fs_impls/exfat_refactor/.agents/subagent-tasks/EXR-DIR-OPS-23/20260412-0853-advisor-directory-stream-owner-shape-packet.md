<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260412-0853-ADVISOR-DIRECTORY-STREAM-OWNER-SHAPE`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260412-0853-advisor-directory-stream-owner-shape-packet.md`
- Supersedes: None
- Role: `advisor`
- Component: `EXR-DIR-OPS-23`
- Phase: `owner-shape analysis`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 08:53 CST`

## Goal

- Reassess whether the current filesystem-owned `directory_stream` bridge is the right final owner shape for `EXR-DIR-OPS-23`, or whether `ExfatInode` should carry an owner-local wrapper or variant without violating owner-first rules.
- Keep this pass analytical only.

## Required Inputs

- Current designer set:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- Current implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- Comparison references:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/ext2/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs/vfs/fs_apis/inode.rs`

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/ext2/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/23_advisor_directory_stream_owner_shape.md`

## Forbidden Scope

- No production-code edits.
- No protocol, board, packet, or handoff edits.
- No command-producing verification.

## Role References To Read

- `$exfat-subagent-workflow`
- `references/advisor.md`

## Questions To Answer

- Is `directory_stream` required to remain an `ExfatFs`-owned bridge, or is there a protocol-compliant shape where `ExfatInode` owns a private wrapper or constructor without inventing a second owner?
- Which parts of the current path are genuinely filesystem-owned because they depend on shared resources or canonical publication state, and which parts are merely convenience placement?
- Does `ExfatInode` frequently upgrading `Arc<ExfatFs>` for directory operations create a meaningful concurrency or ownership problem in the current architecture, or is the concern mostly code-shape friction?
- What narrow follow-up, if any, should the main agent schedule after the current bugfix: no change, designer repair, or creator cleanup?

## Required Output Shape

- A concise diagnosis with:
  - recommended owner shape,
  - why that shape matches the existing protocol and designer intent,
  - the strongest counterargument or tradeoff,
  - one concrete next action recommendation.
- Anchor each conclusion to exact files or line ranges.

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/23_advisor_directory_stream_owner_shape.md`
