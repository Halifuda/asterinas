<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BOOT-34-20260413-1934-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1934-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-BOOT-34`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 19:34 CST`

## Goal

- Land the first `EXR-BOOT-34` creator slice in `fs.rs`: add the owner-private boot-policy snapshot and publication helpers on `ExfatFs`, publish the trusted boot source and persistent boot intent before mount/open exposes the ready root, and keep the row narrow enough that backup parsing, sync ordering, and admin control remain outside scope.

## Architectural Unit Context

- Functional goal: `ExfatFs` boot-region fallback and persistent boot-flag policy
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus owner-private helpers and owner-private state in `fs.rs`
- Parent unit:
  - `EXR-BOOT-34`
- Interfaces served:
  - `ExfatFs::open_root_inode()` as the current mount/open consumer entry
  - future `EXR-SYNC-31` dirty-boot intent consumption
  - the existing `ExfatFs` owner boundary in `fs.rs`

## Required Resolution Questions

- Add an owner-private boot-policy snapshot on `ExfatFs`.
- Publish the trusted boot source and persistent boot intent before `open_root_inode()` publishes the canonical root inode.
- Keep the production mount/open path primary-default unless an owner-private validated fallback candidate is explicitly provided.
- Preserve `VolumeDirty` and `ClearToZero` as boot-region intent, not inode metadata.
- Treat `PercentInUse` as observational only.
  - It is acceptable for the production path to publish `None` when current validated mount facts do not carry an observation yet, as long as the owner-private policy shape leaves room for a later bounded observation input without reopening parsing ownership.
- Keep all new surfaces owner-private to `ExfatFs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Implement against the accepted `EXR-BOOT-34` designer set only.
- Treat the current `fs.rs` mount/open path as the consumer that must start reusing the published policy result.

## Semantic Prior Inputs

- `EXR-BOOT-34` owns only the policy layer above validated boot facts.
- Do not add a second boot parser, backup parser, checksum path, or recovery worker.
- `EXR-FS-OPEN-22` remains the owner of mount/open sequencing.
  - This packet may only make mount/open consume the published boot-policy result.
- `EXR-SYNC-31` remains the owner of filesystem-wide flush ordering.
  - This packet may only publish dirty boot intent for later sync consumption.
- `VolumeDirty` and `ClearToZero` remain persistent boot-region outputs.
- `PercentInUse` remains observational only.

## Integration Prior Inputs

- `ExfatFs` already has:
  - `mount_open_state` as an owner-private mount/open gate
  - `open_root_inode()` as the current mount/open consumer path
  - `super_block.vol_flags_persistent` as the current persistent-flag fact source
- `ExfatSuperBlock` does not currently carry a dedicated published percent-in-use observation for mount/open policy.
  - It is acceptable to model the observation as `Option<u8>` inside the new owner-private policy shape and publish `None` from the production path for now.
- If checker fixtures will clearly need an owner-private way to construct a fallback candidate or a percent-in-use observation later, a small owner-private type in `fs.rs` is allowed.
  - Do not expose it publicly or turn it into a second parser.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the best current low-cost ready row because:
  - it is already fully specified,
  - it does not collide with the new `EXR-RESIZE-37` owner-gap decision in `inode.rs`,
  - and its primary landing zone is `fs.rs` only.
- You are not alone in the codebase.
  - Do not revert or overwrite unrelated edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer owner-private `ExfatFs` state and helpers over module-level free functions.
- Do not widen the row into sync policy, recovery, or admin control.
- Record every new private state carrier, helper, or enum in the creator artifact with its final owner.

## Temporary Interfaces And Exit Plan

- An owner-private fallback-candidate or observation input type in `fs.rs` is allowed if needed to keep future checker fixtures bounded.
  - It must remain owner-private to `ExfatFs`.
  - It must not become a public boot-policy API.
- No background worker, recovery queue, or public boot-control surface is authorized.

## Helper Justification

- Allowed helper surfaces may:
  - choose and publish the trusted boot source,
  - publish `VolumeDirty` and `ClearToZero` as owner-private boot intent,
  - expose the published dirty-boot intent to later `EXR-SYNC-31` consumption through owner-private access,
  - and let `open_root_inode()` reuse the published result instead of re-deciding.
- Reject helpers whose main effect is to:
  - parse or validate boot regions,
  - expose a public boot-policy API,
  - or turn sync into a second decision owner.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay outside `fs.rs`
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - creator lanes for `EXR-SYNC-31` and `EXR-VOLLABEL-35`
  - checker or reviewer lanes for `EXR-BOOT-34`

## Execution Environment

- Host workspace only
- This task is command-free.
  - Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/10_creator_serial.md`
- Do not proceed into checker work.

## Escalation Rule

- If the narrow policy result cannot land without reopening boot parsing ownership, backup validation ownership, or sync ordering ownership, report that exact missing handshake and stop instead of widening scope.
