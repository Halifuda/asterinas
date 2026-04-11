<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-OPEN-22-CHECKER-20260410-1605`
- Date: `2026-04-10`
- Scheduler: `main-agent`
- Skill: `$exfat-subagent-workflow`
- Role: `checker`
- Component: `EXR-FS-OPEN-22`
- Phase: `Serial checker`

## Goal

Validate the repaired `ExfatFs` mount/open sequencing in `fs.rs`.
Own the minimal local `#[ktest]` coverage needed to prove root publication, prerequisite ordering, cache-backed root reuse, and seam removal after the new `open_root_inode(&Arc<Self>)` path landed.
If the first run fails or hangs, use bounded checker-only debug output and/or a debug-oriented `cargo osdk test` rerun inside the allowed write set to localize the issue, then remove temporary debug edits before finishing.

## Required Inputs

- Component architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- Component designer artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`
- Creator artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md`
- Production file to inspect:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Shared testing references:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/11_checker_serial.md`

## Forbidden Scope

- No edits outside the write set.
- No production widening into directory mutation, namespace mutation, allocator policy, file-data behavior, page-cache work, or sync ordering.
- No board edits, handoff edits, packet rewrites, or protocol edits.
- Do not leave temporary debug output or checker-only comments in production files after the final recorded pass.

## Role References To Read

- `$exfat-subagent-workflow`
- `references/common-subagent.md`
- `references/checker.md`
- `references/testing-guide.md`

## This-Round Semantic Constraints

- Preserve `ExfatFs` as the only mount/open owner.
- The root special case must stay distinct from ordinary `InodeKey` entries.
- The checker must validate that root publication, upcase readiness, and bitmap readiness all come from the same owner-local open sequence.
- Do not convert the new path into a manual-publication seam or a separate mount object.

## This-Round Integration Constraints

- Keep new regressions local to `fs.rs`.
- You may add temporary checker-only debug output inside `fs.rs` if the first run fails or hangs, but remove it before the final pass you record.
- You may use a debug-oriented `cargo osdk test` rerun, including `--profile dev`, if needed to diagnose a failure or hang.

## Execution Environment

- Preferred container: `codex-asterinas-dev`
- Repository path in container: `/root/asterinas`
- Preferred command form:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-or-justified-suffix>'`
- Lock acquire command:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire`
- Lock release command:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`

## Allowed Commands

- `sed`, `rg`, `git diff`, `git status`, `cargo fmt --all`
- `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire`
- `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <filter>'`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test --profile dev <filter>'`

## Required Test Coverage

- One regression that proves repeated root access after mount/open returns the canonical published root handle.
- One regression that proves the prerequisite order is enforced: root publication does not happen before upcase and bitmap state become available.
- One regression that proves the root special case remains distinct from ordinary keyed entries while still using the opened-inode reuse boundary.
- One regression that proves the old indefinite `root_inode()` seam is gone once the owner-side open path is used.

## Evidence Requirements

- Record the exact filtered test commands.
- Record whether `/dev/kvm` appeared visible before the run and whether the actual run looked like KVM or TCG.
- Prove that each filter hit the intended test, either by source-backed exact suffixes or by command output listing the executed tests.
- Classify any failure as environment, build, or test failure.
- If you use temporary debug output or debug-profile reruns, record why and confirm the final recorded state removed the temporary debug edits.

## Stop Condition

Stop after one checker artifact:

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/11_checker_serial.md`

The artifact must state one of:

- pass with executable evidence,
- build failure with exact blocker,
- environment failure with exact blocker,
- test failure with exact failing scenario.
