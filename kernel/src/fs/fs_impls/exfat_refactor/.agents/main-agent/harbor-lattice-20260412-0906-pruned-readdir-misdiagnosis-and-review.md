<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `harbor-lattice`
- Date: `2026-04-12 09:06 CST`
- Covered hours: consolidated continuity for the 2026-04-11 owner-cleanup loop and the 2026-04-12 `EXR-DIR-OPS-23` repair, pruning, and review work
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: this note supersedes the previous 2026-04-11 and 2026-04-12 main-agent handoffs for the current read-path wave; `EXR-DIR-OPS-23` now has landed implementation plus a no-findings review, `EXR-FILE-MAP-24` is specified, and the stale `readdir` misdiagnosis chain has been removed

## Environment Summary

- Shared checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- Use `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <suffix>'` for exact local reruns.
- `qemu-serial.log` is the primary diagnosis source for opaque `cargo osdk test ... -> exit 1`; terminal output and `qemu.log` are not enough by themselves.
- Early serial lines such as `WARNING: no console will be available to OS` and `error: no suitable video mode found.` did not prevent guest ktests from completing in the validated local reruns.

## Current Project State

- Current goal:
  - close the current read-side wave cleanly and move on to regular-file mapping
- Current phase:
  - `EXR-DIR-OPS-23` implementation landed, reviewed, and continuity-normalized; `EXR-FILE-MAP-24` is ready for creator work once `23` is formally closed
- Active or next component:
  - close `EXR-DIR-OPS-23`, then start `EXR-FILE-MAP-24`
- Latest accepted components:
  - `EXR-FS-OPEN-22` remains accepted
  - all rows through `EXR-BITMAP-21` remain accepted
- Components in progress:
  - `EXR-DIR-OPS-23` is still `Reviewing` on the board, but reviewer already returned `No findings`
  - `EXR-FILE-MAP-24` is `Specified`
- Blocked components:
  - none

## Active Work Slice Matrix

There are no active delegated lanes at handoff time.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-DIR-OPS-23-CURRENT` | `EXR-DIR-OPS-23` | Read-only directory `lookup` / `readdir_at` over `DirectoryEngine`, with the invalid readdir misdiagnosis chain already removed | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`, `.agents/components/EXR-DIR-OPS-23/30_reviewer_report.md` | accepted `EXR-FS-OPEN-22`, accepted `EXR-DIR-ENGINE-19`, accepted `EXR-UPCASE-20` | future `EXR-FILE-MAP-24` creator should wait until `23` is formally closed | implementation plus review complete | no active lane | `.agents/components/EXR-DIR-OPS-23/30_reviewer_report.md` | `.agents/subagent-tasks/EXR-DIR-OPS-23/20260412-0904-reviewer-packet.md` |
| `WS-FILE-MAP-24-NEXT` | `EXR-FILE-MAP-24` | Regular-file logical-to-physical mapping helpers in `ExfatInode` | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` plus future artifacts | accepted `EXR-CHAIN-03B`, accepted `EXR-INODE-CORE-17`, specified `EXR-FILE-MAP-24` designer set | should start only after `EXR-DIR-OPS-23` is formally closed | next planned creator lane | not started | `.agents/components/EXR-FILE-MAP-24/01_designer_core.md` | `.agents/subagent-tasks/EXR-FILE-MAP-24/20260411-1613-designer-packet.md` |

## Recent Decisions

- `EXR-FS-OPEN-22` received one narrow post-accept owner-first cleanup: `read_chain_bytes` moved into a private `ExfatFs` method, and the targeted root-mount-sequence ktest passed.
- `EXR-FILE-MAP-24` was designed and is ready for creator work.
- `EXR-DIR-OPS-23` needed a designer repair before creator could land the correct owner-facing bridges; the surviving implementation now lives across `inode.rs`, `fs.rs`, and `directory.rs`.
- The first apparent `readdir_*` failure was not a production `readdir_at` bug. The real immediate cause was test-owned: the tests dropped the owning `Arc<ExfatFs>` before calling `root.readdir_at(...)`.
- The two `readdir_*` ktests were repaired to keep `Arc<ExfatFs>` alive, and direct exact local reruns passed.
- The stale 2026-04-11 `readdir` repair chain that grew out of opaque checker output was removed from the component history.
- Review then ran on the surviving `EXR-DIR-OPS-23` implementation and returned `No findings`.
- Current owner-shape recommendation: keep `directory_stream` filesystem-owned in `ExfatFs`; if cleanup is desired later, allow only a thin inode-private wrapper.

## Wave Record

- The repo-local creator and reviewer guidance was tightened during this wave so owner-first landing-form checks happen earlier and more explicitly.
- Checker guidance was tightened to require inspection of `qemu-serial.log` when QEMU-backed tests exit nonzero without a clear guest panic in terminal output.
- The surviving `EXR-DIR-OPS-23` artifact set is:
  - design: `00`, `01`, `02`, `03`
  - implementation: `10`, `12`
  - test-owned lifetime repair: `22`
  - owner-shape analysis: `23`
  - review: `30`
- The deleted `readdir` repair chain must not be resurrected.

## Open Risks And Assumptions

- `EXR-DIR-OPS-23` still lacks a fresh canonical checker artifact after the pruning step; the runtime evidence currently trusted is the exact local reruns for the two `readdir_*` tests.
- Because of that, the board is still `Reviewing` rather than `Accepted`.
- `EXR-FILE-MAP-24` should not start until `EXR-DIR-OPS-23` is formally closed, because both rows want `inode.rs`.

## Recommended Next Actions

1. Archive one fresh minimal checker artifact for `EXR-DIR-OPS-23` that cites the already-proven exact local reruns and current `qemu-serial.log` evidence, then move the row toward closure.
2. Once `EXR-DIR-OPS-23` is formally closed, start the `EXR-FILE-MAP-24` creator lane.
3. Do not reopen `readdir` diagnosis unless a new failing exact test produces fresh evidence beyond the already-fixed owner-lifetime bug.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this note as the consolidated handoff for the current read-path wave.
- Before dispatching new work on `EXR-DIR-OPS-23`, remember that the deleted `readdir` repair chain is intentionally invalidated history.
