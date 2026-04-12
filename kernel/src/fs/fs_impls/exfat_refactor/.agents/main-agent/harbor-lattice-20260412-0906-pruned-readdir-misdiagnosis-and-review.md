<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `harbor-lattice`
- Date: `2026-04-12 10:35 CST`
- Covered hours: consolidated continuity for the 2026-04-11 owner-cleanup loop and the 2026-04-12 `EXR-DIR-OPS-23` closure, `EXR-FILE-MAP-24` creator launch, and `EXR-READ-OPS-25` architect pre-research
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: this note supersedes the earlier 2026-04-12 closure draft for the current read-path wave; `EXR-DIR-OPS-23` is now formally accepted on documented skip reasoning, `EXR-FILE-MAP-24` has landed its serial creator pass, and `EXR-READ-OPS-25` is architected

## Environment Summary

- Shared checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- Use `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <suffix>'` for exact local reruns.
- `qemu-serial.log` is the primary diagnosis source for opaque `cargo osdk test ... -> exit 1`; terminal output and `qemu.log` are not enough by themselves.
- Early serial lines such as `WARNING: no console will be available to OS` and `error: no suitable video mode found.` did not prevent guest ktests from completing in the validated local reruns.

## Current Project State

- Current goal:
  - move from directory-read closure into regular-file mapping verification and buffered-read planning
- Current phase:
  - `EXR-DIR-OPS-23` is formally closed; `EXR-FILE-MAP-24` has landed its serial creator pass and awaits checker work; `EXR-READ-OPS-25` now has an architected owner boundary
- Active or next component:
  - prepare `EXR-FILE-MAP-24` checker work, then advance `EXR-READ-OPS-25` into designer work
- Latest accepted components:
  - `EXR-DIR-OPS-23` is now accepted
  - `EXR-FS-OPEN-22` remains accepted
  - all rows through `EXR-BITMAP-21` remain accepted
- Components in progress:
  - `EXR-FILE-MAP-24` is `SerialImplementing`
  - `EXR-READ-OPS-25` is `Architected`
- Blocked components:
  - none

## Active Work Slice Matrix

There are no active delegated lanes at handoff time.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FILE-MAP-24-CURRENT` | `EXR-FILE-MAP-24` | Regular-file logical-to-physical mapping helpers in `ExfatInode` | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-FILE-MAP-24/10_creator_serial.md` | accepted `EXR-CHAIN-03B`, accepted `EXR-INODE-CORE-17`, specified `EXR-FILE-MAP-24` designer set | future `EXR-READ-OPS-25` designer work may overlap only as artifact-only planning | serial creator landed; checker next | no active lane | `.agents/components/EXR-FILE-MAP-24/10_creator_serial.md` | `.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1015-creator-serial-packet.md` |
| `WS-READ-OPS-25-ARCH-20260412` | `EXR-READ-OPS-25` | Name the stable `ExfatInode` buffered `read_at` owner boundary that consumes file mapping without widening into page-cache ownership | `.agents/components/EXR-READ-OPS-25/00_architect.md` | current `EXR-FILE-MAP-24` boundary contract | future `EXR-FILE-MAP-24` checker and review lanes because the write set is artifact-only | architect returned and accepted | no active lane | `.agents/components/EXR-READ-OPS-25/00_architect.md` | `.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1018-architect-packet.md` |

## Recent Decisions

- `EXR-FS-OPEN-22` received one narrow post-accept owner-first cleanup: `read_chain_bytes` moved into a private `ExfatFs` method, and the targeted root-mount-sequence ktest passed.
- `EXR-DIR-OPS-23` needed a designer repair before creator could land the correct owner-facing bridges; the surviving implementation now lives across `inode.rs`, `fs.rs`, and `directory.rs`.
- The first apparent `readdir_*` failure was not a production `readdir_at` bug. The real immediate cause was test-owned: the tests dropped the owning `Arc<ExfatFs>` before calling `root.readdir_at(...)`.
- The two `readdir_*` ktests were repaired to keep `Arc<ExfatFs>` alive, and direct exact local reruns passed.
- The stale 2026-04-11 `readdir` repair chain that grew out of opaque checker output was removed from the component history.
- Review then ran on the surviving `EXR-DIR-OPS-23` implementation and returned `No findings`.
- `EXR-DIR-OPS-23` is now accepted without a new checker or final-checker rerun because the surviving late repair was test-only, the reviewer made no production edits, and the preserved exact local rerun evidence remains the last executable proof in the closure chain.
- Current owner-shape recommendation: keep `directory_stream` filesystem-owned in `ExfatFs`; if cleanup is desired later, allow only a thin inode-private wrapper.
- `EXR-FILE-MAP-24` creator then landed owner-private mapping helpers and a small `PhysicalFileRange` result in `inode.rs` without touching buffered read policy, zero-fill policy, or page-cache ownership.
- The current `EXR-FILE-MAP-24` implementation still uses explicit caller-supplied traversal context (`&dyn BlockDevice`, `&ExfatSuperBlock`) because this creator pass stayed inside `inode.rs`; that temporary surface is recorded for later checker/reviewer validation rather than widened into `fs.rs`.
- `EXR-READ-OPS-25` architect pre-research returned in parallel and keeps buffered byte transfer, EOF/short-read behavior, and valid-size zero-fill under `ExfatInode` while leaving translation in `EXR-FILE-MAP-24` and page-cache integration in `EXR-PGCACHE-26`.

## Wave Record

- The repo-local creator and reviewer guidance was tightened during this wave so owner-first landing-form checks happen earlier and more explicitly.
- Checker guidance was tightened to require inspection of `qemu-serial.log` when QEMU-backed tests exit nonzero without a clear guest panic in terminal output.
- The surviving `EXR-DIR-OPS-23` artifact set is:
  - design: `00`, `01`, `02`, `03`
  - implementation: `10`, `12`
  - test-owned lifetime repair: `22`
  - owner-shape analysis: `23`
  - review: `30`
- The `EXR-DIR-OPS-23` closure decision is now recorded at main-agent level instead of creating a fresh checker artifact, because the only surviving post-rerun change was test-only and the reviewer made no production edits.
- `EXR-FILE-MAP-24` creator work was launched with component-local priors only and intentionally did not carry `EXR-DIR-OPS-23` history into the packet.
- `EXR-READ-OPS-25` architect pre-research completed in parallel with the `EXR-FILE-MAP-24` creator lane.
- The deleted `readdir` repair chain must not be resurrected.

## Open Risks And Assumptions

- `EXR-FILE-MAP-24` has not been checker-verified yet; the temporary explicit traversal-context arguments should receive bounded checker and reviewer attention so they do not silently become a long-lived owner leak.
- `EXR-DIR-OPS-23` acceptance now relies on recorded skip reasoning plus preserved rerun evidence rather than a canonical post-prune checker artifact; future reopen should require fresh failing evidence, not the deleted misdiagnosis chain.
- `EXR-READ-OPS-25` is only architected so far; designer work still needs to pin the buffered-read boundary around EOF, short-read, and valid-size zero-fill details without drifting into page cache.

## Recommended Next Actions

1. Archive an `EXR-FILE-MAP-24` checker packet that validates helper behavior in `inode.rs`, especially empty/out-of-range mapping results, cluster-boundary caps, repeated-call stability, and the temporary explicit traversal-context arguments.
2. If the `EXR-FILE-MAP-24` checker returns cleanly, run bounded review on the current mapping helper surface and decide whether a final checker rerun can be skipped.
3. Start `EXR-READ-OPS-25` designer work once the `EXR-FILE-MAP-24` implementation contract is stable enough for designer reuse.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this note as the consolidated handoff for the current read-path wave.
- Treat `EXR-DIR-OPS-23` as formally closed unless fresh failing evidence appears; the deleted `readdir` repair chain remains intentionally invalidated history.
- Resume `EXR-FILE-MAP-24` from the landed creator artifact and the temporary traversal-context note rather than reopening directory-side history.
