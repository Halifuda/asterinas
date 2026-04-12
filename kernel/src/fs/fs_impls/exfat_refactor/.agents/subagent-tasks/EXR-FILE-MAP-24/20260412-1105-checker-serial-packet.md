<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FILE-MAP-24-20260412-1105-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1105-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-FILE-MAP-24`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 11:05 CST`

## Goal

- Validate the new `ExfatInode` mapping helpers in `inode.rs`, add the required local ktests, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-path logical-to-physical file mapping
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-private helpers in `inode.rs`
- Parent units:
  - `EXR-INODE-CORE-17`
  - `EXR-CHAIN-03B`

## Required Resolution Questions

- Verify one logical offset maps to the expected cluster position and in-cluster byte offset for a regular file.
- Verify the physically mappable span is bounded by file size, valid size, allocated size, and cluster geometry.
- Verify requests that start at or beyond the physically backed region stay explicit rather than silently inventing read policy.
- Verify repeated calls on the same inode snapshot return the same translation result.
- Evaluate the current explicit `&dyn BlockDevice` and `&ExfatSuperBlock` helper arguments as a temporary surface: keep them only if the current packet-scoped boundary justifies them; otherwise report the issue rather than widening into `fs.rs`.
- If a compile/test failure is strictly local to `inode.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- Use the accepted `EXR-FILE-MAP-24` designer constraints only.
- This row owns translation plus physically mappable span only. Do not turn the checker into buffered-read, zero-fill, EOF, or page-cache validation.
- Treat the explicit traversal-context arguments as a recorded temporary surface, not as permission to move mapping ownership into `fs.rs`.

## Integration Prior Inputs

- `EXR-INODE-CORE-17` still owns the explicit `read_at` seam; keep it explicit.
- Tests should stay local to `inode.rs` and validate mapping behavior through `ExfatInode`, not by reopening directory behavior or mount/open sequencing.

## Workflow Prior Inputs

- Runtime/test-producing checker lane
- Command-producing verification must hold the checker execution lock
- Add local `#[ktest]` coverage in `inode.rs` with a stable source-backed filter prefix. Prefer test names starting with `file_mapping_`.
- Temporary ktest-local debug output is allowed only if needed to surface a failure and must be removed before stopping unless the final artifact records why it remains.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.
- Prefer test-only edits unless a strictly local production fix is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep mapping helpers owner-private to `ExfatInode`.
- Do not add a public mapping service, filesystem-global traversal accessor, or buffered-read shell.
- If the explicit traversal-context arguments remain after checker, record why they are acceptable for now and what later owner should absorb them.

## Helper Justification

- New helper changes are justified only when they keep mapping translation local to `ExfatInode` or keep the checker-owned local tests readable.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_'`
- If the standard filtered run does not surface enough evidence, one or more additional debug-oriented reruns of the same filtered tests are allowed with extra verbosity or other non-scope-widening `cargo osdk test` flags. Record the exact command if used.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `inode.rs`

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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-FILE-MAP-24 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`

## Escalation Rule

- If the checker needs edits outside `inode.rs` or cannot get trustworthy evidence from the allowed filtered commands, report that and stop.
