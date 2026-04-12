<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `maple-anchor`
- Date: `2026-04-12 20:49 CST`
- Covered hours: continuation after `harbor-lattice`; `EXR-FILE-MAP-24` checker/reviewer closure, `EXR-READ-OPS-25` full creator/checker/reviewer closure, `EXR-PGCACHE-26` / `EXR-ALLOC-27` designer completion, `EXR-DENTRY-WRITE-28` architect return, and the next `EXR-PGCACHE-26` creator plus `EXR-DENTRY-WRITE-28` designer dispatch
- Author: `main-agent`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: current continuity note for the post-`EXR-DIR-OPS-23` read-path wave; `EXR-FILE-MAP-24` and `EXR-READ-OPS-25` are now accepted, `EXR-PGCACHE-26` has an active creator lane, `EXR-ALLOC-27` is specified, and `EXR-DENTRY-WRITE-28` has an active designer lane after architect closure

## Environment Summary

- Shared checker execution is still serialized through `.agents/tools/checker_lock.sh`.
- Use `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <suffix>'` for exact local reruns.
- `qemu-serial.log` remains the primary diagnosis source for opaque `cargo osdk test ... -> exit 1`.

## Current Project State

- Current goal:
  - turn the accepted buffered regular-file read path into active inode-local cache work while continuing write-side planning on the directory-mutation boundary
- Current phase:
  - `EXR-FILE-MAP-24` is accepted after checker and reviewer closure
  - `EXR-READ-OPS-25` is accepted after creator, checker, and no-findings review closure
  - `EXR-PGCACHE-26` has an active serial creator lane
  - `EXR-ALLOC-27` is specified
  - `EXR-DENTRY-WRITE-28` has an active designer lane
- Latest accepted components:
  - `EXR-READ-OPS-25`
  - `EXR-FILE-MAP-24`
  - `EXR-DIR-OPS-23`
  - `EXR-FS-OPEN-22`
  - all rows through `EXR-BITMAP-21`
- Components in progress:
  - `EXR-PGCACHE-26` is `Specified` on the board, with `20260412-2049-creator-serial-packet.md` in flight
  - `EXR-ALLOC-27` is `Specified`
  - `EXR-DENTRY-WRITE-28` is `Architected` on the board, with `20260412-2049-designer-packet.md` in flight
- Blocked components:
  - none

## Active Work Slice Matrix

Two delegated lanes are active at handoff update time: one creator lane on the accepted read-side continuation, and one command-free designer lane on the newly architected directory-write row.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-PGCACHE-26-CREATE` | `EXR-PGCACHE-26` | Land inode-local `PageCache` ownership and `PageCacheBackend` integration on `ExfatInode` while reusing accepted buffered-read behavior and deferring dirty persistence | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-PGCACHE-26/10_creator_serial.md` | accepted `EXR-READ-OPS-25`, accepted `EXR-FILE-MAP-24` | command-free planning lanes with disjoint write sets | command-free production edit | dispatched / in progress | `.agents/components/EXR-PGCACHE-26/01_designer_core.md` | `.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2049-creator-serial-packet.md` |
| `WS-DENTRY-WRITE-28-DESIGN` | `EXR-DENTRY-WRITE-28` | Turn the architected directory-write mutation boundary into a creator-ready designer set that consumes validated file-record sets and committed allocation results without absorbing namespace policy | `.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`, `.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`, `.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md` | architected `EXR-DENTRY-WRITE-28`, specified `EXR-ALLOC-27` | active `EXR-PGCACHE-26` creator because the write sets are disjoint | command-free planning | dispatched / in progress | `.agents/components/EXR-DENTRY-WRITE-28/00_architect.md` | `.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md` |
| `WS-READ-OPS-25-CREATE` | `EXR-READ-OPS-25` | Land buffered regular-file `read_at` on `ExfatInode`, including EOF truncation, short-read accounting, valid-size zero-fill, and the narrow traversal-context seam needed to consume accepted file mapping | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-READ-OPS-25/10_creator_serial.md` | accepted `EXR-FILE-MAP-24`, accepted `EXR-INODE-CORE-17` | later checker prep and future artifact-only planning lanes with disjoint write sets | command-free production edit | returned and integrated | `.agents/components/EXR-READ-OPS-25/10_creator_serial.md` | `.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1202-creator-serial-packet.md` |
| `WS-READ-OPS-25-CHECK` | `EXR-READ-OPS-25` | Add the checker-owned buffered-read ktests, run exact filtered verification under the checker lock, and record evidence for the new read-path owner | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-READ-OPS-25/11_checker_serial.md` | `WS-READ-OPS-25-CREATE`, accepted `EXR-FILE-MAP-24` | command-free planning lanes only because this is the serialized checker command lane | runtime/test-producing | returned and integrated | `.agents/components/EXR-READ-OPS-25/11_checker_serial.md` | `.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1214-checker-serial-packet.md` |
| `WS-READ-OPS-25-REVIEW` | `EXR-READ-OPS-25` | Review the serial-checked buffered read row for owner-boundary drift, local correctness risks, and temporary-surface hygiene without reopening runtime verification | `.agents/components/EXR-READ-OPS-25/30_reviewer_report.md` | `WS-READ-OPS-25-CHECK`, accepted `EXR-FILE-MAP-24` | other command-free lanes only | command-free review | returned and integrated | `.agents/components/EXR-READ-OPS-25/30_reviewer_report.md` | `.agents/subagent-tasks/EXR-READ-OPS-25/20260412-2046-reviewer-packet.md` |
| `WS-PGCACHE-26-DESIGN` | `EXR-PGCACHE-26` | Turn the inode-local page-cache architect boundary into a creator-ready designer set that consumes buffered read ownership without re-owning read policy or dirty writeback | `.agents/components/EXR-PGCACHE-26/01_designer_core.md`, `.agents/components/EXR-PGCACHE-26/02_designer_async.md`, `.agents/components/EXR-PGCACHE-26/03_designer_ktest.md` | architected `EXR-PGCACHE-26`, specified `EXR-READ-OPS-25` design | active `EXR-READ-OPS-25` creator because the write sets are disjoint | command-free planning | returned and integrated | `.agents/components/EXR-PGCACHE-26/01_designer_core.md` | `.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1202-designer-packet.md` |
| `WS-ALLOC-27-DESIGN` | `EXR-ALLOC-27` | Turn the filesystem-owned allocator architect boundary into a creator-ready designer set covering search, reservation intent, and bitmap/FAT commit without drifting into namespace or sync policy | `.agents/components/EXR-ALLOC-27/01_designer_core.md`, `.agents/components/EXR-ALLOC-27/02_designer_async.md`, `.agents/components/EXR-ALLOC-27/03_designer_ktest.md` | architected `EXR-ALLOC-27`, accepted `EXR-BITMAP-21`, accepted `EXR-FATVAL-03A`, accepted `EXR-IO-02` | active `EXR-READ-OPS-25` creator because the write sets are disjoint | command-free planning | returned and integrated | `.agents/components/EXR-ALLOC-27/01_designer_core.md` | `.agents/subagent-tasks/EXR-ALLOC-27/20260412-1202-designer-packet.md` |
| `WS-DENTRY-WRITE-28-ARCH` | `EXR-DENTRY-WRITE-28` | Name the stable write-side `DirectoryEngine` mutation boundary that consumes validated file-record sets and committed allocation results without absorbing namespace policy or sync ordering | `.agents/components/EXR-DENTRY-WRITE-28/00_architect.md` | accepted `EXR-DIR-ENGINE-19`, accepted `EXR-FILESET-04B`, specified `EXR-ALLOC-27` | active `EXR-READ-OPS-25` checker because the write sets are disjoint and this lane is command-free | command-free planning | returned and integrated | `.agents/components/EXR-DENTRY-WRITE-28/00_architect.md` | `.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-1217-architect-packet.md` |
| `WS-FILE-MAP-24-CLOSED` | `EXR-FILE-MAP-24` | Close regular-file mapping after creator, serial checker, and no-findings review | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`, `.agents/components/EXR-FILE-MAP-24/30_reviewer_report.md` | accepted `EXR-CHAIN-03B`, accepted `EXR-INODE-CORE-17` | future `EXR-READ-OPS-25` creator can now start | creator + checker + reviewer complete | closed | `.agents/components/EXR-FILE-MAP-24/30_reviewer_report.md` | `.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1105-checker-serial-packet.md`, `.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1107-reviewer-packet.md` |

## Recent Decisions

- `EXR-FILE-MAP-24` checker completed inside packet boundary: it added four local `file_mapping_*` ktests in `inode.rs`, corrected the too-broad initial filter by rerunning four exact test names under the same checker lock, and returned `No findings`.
- `EXR-FILE-MAP-24` reviewer then returned `No findings` without any production edits.
- `EXR-FILE-MAP-24` is now accepted without a final checker rerun because the required serial checker passed, the reviewer made no production edits, and the remaining temporary seam is explicitly recorded with a later owner/removal condition.
- `EXR-READ-OPS-25` designer work is now complete and recorded in `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`.
- The `EXR-READ-OPS-25` design explicitly consumes the current `EXR-FILE-MAP-24` helper contract, including the temporary explicit traversal-context arguments, but keeps buffered read ownership on `ExfatInode`.
- EOF truncation, short-read accounting, and valid-size zero-fill are now pinned to `EXR-READ-OPS-25`; they must not be pushed back into `EXR-FILE-MAP-24`.
- The main agent archived and dispatched `EXR-READ-OPS-25` creator, `EXR-PGCACHE-26` designer, and `EXR-ALLOC-27` designer packets together at `2026-04-12 12:02 CST`.
- `EXR-READ-OPS-25` creator work is now complete and recorded in `10_creator_serial.md`.
- The creator landed the buffered regular-file read loop on `ExfatInode`, kept mapping translation subordinate to `EXR-FILE-MAP-24`, and added a thin temporary `ExfatFs::file_read_context()` seam so the inode-owned path can source traversal context without becoming a reader service.
- The local inode smoke ktest in `inode.rs` now exercises real buffered byte transfer instead of the retired `read_at` `EOPNOTSUPP` seam.
- The main agent archived and dispatched the `EXR-READ-OPS-25` serial checker packet at `2026-04-12 12:14 CST`.
- `EXR-PGCACHE-26` designer work is now complete and recorded in `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`.
- The `EXR-PGCACHE-26` design keeps `PageCache` ownership on `ExfatInode`, reuses `EXR-READ-OPS-25` for page-fill semantics, and explicitly defers `write_page_async` persistence semantics to later write/sync owners.
- `EXR-ALLOC-27` designer work is now complete and recorded in `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`.
- The `EXR-ALLOC-27` design keeps search, reservation intent, and bitmap/FAT commit under `ExfatFs` while publishing only a small committed result shape for later namespace/write owners.
- `EXR-DENTRY-WRITE-28` architect work is now complete and recorded in `00_architect.md`.
- The `EXR-DENTRY-WRITE-28` architect artifact keeps write-side directory slot discovery, record placement/removal, tombstoning, and overwrite policy under `DirectoryEngine` while leaving namespace decisions to later `ExfatInode` owners.
- `EXR-READ-OPS-25` checker work is now complete and recorded in `11_checker_serial.md`.
- The checker added the four dedicated `file_buffered_read_*` regressions in `inode.rs`, repaired one strictly local `fill_zeros()` error-type mismatch in `zero_fill_valid_size_gap()`, and then passed exact-name reruns for all four designer scenarios under the checker lock.
- The main agent archived and dispatched the `EXR-READ-OPS-25` reviewer packet at `2026-04-12 20:46 CST`.
- `EXR-READ-OPS-25` reviewer work is now complete and recorded in `30_reviewer_report.md`.
- The reviewer reported `No findings`, kept the lane report-only, and did not edit production code.
- `EXR-READ-OPS-25` is now accepted without a final checker rerun because the serial checker already passed with exact-name proof, the reviewer made no production edits, and the temporary `file_read_context()` seam remained a narrow recorded bridge rather than a new owner surface.
- The main agent archived and dispatched the `EXR-PGCACHE-26` creator packet at `2026-04-12 20:49 CST`.
- The main agent archived and dispatched the `EXR-DENTRY-WRITE-28` designer packet at `2026-04-12 20:49 CST`.

## Open Risks And Assumptions

- `EXR-READ-OPS-25` now depends on the current explicit traversal-context arguments from `EXR-FILE-MAP-24`; checker and reviewer both accepted that thin seam for now, but later cache/read consolidation should still absorb it rather than letting it ossify into a long-lived reader API.
- `EXR-PGCACHE-26` is now specified against the `EXR-READ-OPS-25` designer boundary while the creator lane is still active. When cache creator work starts, it must consume the final checked buffered-read implementation rather than silently assuming today's pre-check seam shape.
- `EXR-ALLOC-27` should stay filesystem-owned. If later creator work drifts toward inode-local allocation helpers or sync/writeback ownership, stop and reslice.
- `EXR-DENTRY-WRITE-28` is only architected so far. Later designer work must keep file-record validation on `EXR-FILESET-04B` and allocation ownership on `EXR-ALLOC-27` instead of letting `directory.rs` become a catch-all mutation layer.

## Recommended Next Actions

1. Integrate the `EXR-PGCACHE-26` creator return, then either move the row into checker or reslice if the lane stop-reports a missing cache-boundary handshake.
2. Integrate the `EXR-DENTRY-WRITE-28` designer return so the directory-write row can move from architected to specified.
3. Keep `EXR-ALLOC-27` as the next write-side creator-ready alternative once the scheduler chooses between cache-first and allocator-first production work.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this note after `harbor-lattice` for the latest post-dispatch scheduling state.
- Treat `EXR-FILE-MAP-24` as accepted unless fresh failing evidence appears; the current temporary traversal-context seam is recorded and reviewer-approved for now.
- Treat `EXR-READ-OPS-25` as accepted unless fresh failing evidence appears; the current `file_read_context()` seam is recorded as a temporary narrow bridge already accepted by checker and reviewer.
- Treat `EXR-PGCACHE-26` as specified on the board with active creator packet `.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2049-creator-serial-packet.md`.
- Treat `EXR-ALLOC-27` as specified and creator-ready once the scheduler chooses the next write-side wave.
- Treat `EXR-DENTRY-WRITE-28` as architected on the board with active designer packet `.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md`.
