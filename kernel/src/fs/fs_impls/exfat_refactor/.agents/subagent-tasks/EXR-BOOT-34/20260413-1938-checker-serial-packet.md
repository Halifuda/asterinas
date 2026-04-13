<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BOOT-34-20260413-1938-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1938-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-BOOT-34`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 19:38 CST`

## Goal

- Check the landed `EXR-BOOT-34` creator slice in `fs.rs`: prove the owner-private boot-policy snapshot, primary-default trusted source publication, persistent dirty-boot intent, and observational `percent_in_use` stance with exact-name local `fs.rs` tests, and classify any strictly local issue inside `fs.rs`.

## Architectural Unit Context

- Functional goal: `ExfatFs` boot-region fallback and persistent boot-flag policy
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus owner-private helpers and owner-private state in `fs.rs`

## Required Resolution Questions

- Prove that the boot-policy snapshot is published once before the ready root is exposed.
- Prove that the current production path remains primary-default when no fallback candidate is provided.
- Prove that persistent dirty boot intent stays separate from the trusted-source decision.
- Prove that `percent_in_use` remains observational and does not perturb the published trusted source or dirty intent.
- If checker finds a strictly local `fs.rs` bug or needs compact local `#[ktest]` coverage to prove the landed slice, make the narrowest in-scope fix and record it.
- Do not widen into backup parsing, sync ordering, or public boot-policy APIs.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- reviewer, advisor, and sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- `EXR-BOOT-34` owns policy publication only; it does not own boot parsing or sync ordering.
- The current creator slice stays primary-default in production and keeps `percent_in_use` as an optional observation slot.
- `EXR-SYNC-31` is not implemented yet.
  - For this checker pass, the sync-handoff obligation is satisfied by the owner-private dirty-intent projection in `fs.rs`; do not widen into implementing `sync()` policy.
- `ClearToZero` currently rides the second persistent boot-region flag bit in the owner-private snapshot carrier; treat that as the current staged representation rather than demanding a second parser or a separate flag source in this pass.

## Integration Prior Inputs

- The landed creator slice added owner-private carriers in `fs.rs`:
  - `BootSource`
  - `BootDirtyIntent`
  - `BootPolicySnapshot`
  - `BootPolicyState`
- The landed creator slice publishes the boot-policy snapshot from `ExfatFs::open_root_inode()` before the ready root is exposed.
- The current production path does not yet consume a real validated backup fact bundle; fallback is an owner-private future path only.
- It is acceptable for checker to add local `#[ktest]` coverage in `fs.rs` that exercises owner-private helpers directly when the policy row has no wider runtime consumer yet.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Record whether execution used KVM or fell back to TCG.
- Exact test names are required; do not use fragments.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer test-only edits unless a strictly local `fs.rs` production fix is necessary.
- Reject any solution that turns the row into backup parsing, a background recovery worker, or a second sync owner.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_primary_source_is_published_before_root_open'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_fallback_selection_stays_owner_private'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_persistent_dirty_intent_stays_separate_from_source'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_policy_percent_in_use_is_observational_only'`
- If the guest-side failure is unclear after a nonzero run, read-only inspection of `/home/halifuda/asterinas/qemu-serial.log` is allowed.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Execution Environment

- Host and Docker
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-BOOT-34 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md`

## Escalation Rule

- If trustworthy proof would require edits outside `fs.rs`, or if checking the row would require implementing real backup parsing or real sync ordering, report that exact missing handshake and stop instead of widening scope.
