<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `harbor-signal`
- Date: 2026-04-10 11:20 CST
- Covered hours: unified 2026-04-10 main-agent continuity record, merging only today's handoff checkpoints into this file
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: shared host workspace plus shared Docker container `codex-asterinas-dev`
- Status: `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`, `EXR-INODE-CACHE-18`, `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, `EXR-BITMAP-21`, and `EXR-FS-OPEN-22` are accepted; `EXR-DIR-OPS-23` is specified; and `EXR-FILE-MAP-24` is architected and parked for the next loop

## Historical Continuity

This file is now the single surviving handoff for the current date (`2026-04-10`).
Only today's earlier checkpoints were merged into this record to reduce same-day resume noise.
Older historical handoffs from previous dates remain preserved in `.agents/main-agent/`.

Merged same-day predecessor checkpoints:

- `cinder-harbor` (`2026-04-10 11:02`): 16/17 reviewers complete, 20/21 designers complete, 18/19 creator round active.
- `amber-flare` (`2026-04-10 11:15`): repaired checker environment for 16/17, including `initramfs` rebuild and `tools/qemu_args.sh` executable-bit recovery.

The essential carried-forward facts from those merged same-day files are:

- environment continuity matters: checker runs are serialized, `/dev/kvm` visibility is inconsistent in practice, TCG-backed evidence is common, `test/initramfs/build/` and `tools/qemu_args.sh` mode must be watched;
- the repaired 16/17 checker evidence from earlier today is already incorporated here, so future resumes do not need to bounce between multiple same-day handoffs.

## Environment Summary

- Image or base environment: shared workspace with host-managed Codex session and shared Docker container
- Working path: `/home/halifuda/asterinas`
- Container name, if any: `codex-asterinas-dev`
- KVM status:
  - `/dev/kvm` was visible during the repaired 16/17 checker reruns
  - practical QEMU output still showed TCG CPU-feature warnings, so treat current evidence as TCG-backed unless a later run proves otherwise
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <filtered suffix>'`
- Known environment blockers:
  - keep watching `tools/qemu_args.sh` mode and `test/initramfs/build/` before future checker work

## Current Project State

- Current goal: close the 22 mount/open row cleanly after the 2026-04-11 local checker-repair loop and leave the next read-side rows staged for the following creator/planning loop
- Current phase:
  - `EXR-FS-CORE-16`: accepted after repaired checker evidence and reviewer pass with no production edits
  - `EXR-INODE-CORE-17`: accepted after repaired checker evidence and reviewer pass with no production edits
  - `EXR-INODE-CACHE-18`: accepted after retry checker evidence and reviewer pass with no production edits
  - `EXR-DIR-ENGINE-19`: accepted after reviewer hardening plus passing final checker
  - `EXR-UPCASE-20`: accepted after creator, checker, and reviewer completion with no review-time production edits
  - `EXR-BITMAP-21`: accepted after a clean reviewer pass with no production edits
  - `EXR-FS-OPEN-22`: accepted after a checker-driven local repair, passing retry checker evidence, and a clean reviewer pass
  - `EXR-DIR-OPS-23`: specified after accepted architect and designer artifacts
  - `EXR-FILE-MAP-24`: architect accepted locally; row parked at `Architected`
- Active or next component:
  - No component currently has an active writer.
  - The next natural single creator round is `EXR-DIR-OPS-23`, while `EXR-FILE-MAP-24` can advance through an artifact-only designer lane in parallel.
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
  - `EXR-BOOTTYPE-14`
  - `EXR-SBGEOM-15`
  - `EXR-FS-CORE-16`
  - `EXR-INODE-CORE-17`
  - `EXR-INODE-CACHE-18`
  - `EXR-DIR-ENGINE-19`
  - `EXR-UPCASE-20`
  - `EXR-BITMAP-21`
  - `EXR-FS-OPEN-22`
- Components in progress:
  - none
- Blocked components:
  - none formally blocked

## Active Work Slice Matrix

This is the scheduler-owned global view of the currently active loop.
The table itself records the 2026-04-10 live slice layout; the 2026-04-11 local follow-up closed `EXR-FS-OPEN-22`, so there is no active writer at the moment.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-BITMAP-21-CREATE-20260410` | `EXR-BITMAP-21` | Implement the owner-private validated allocation bitmap plus read-only occupancy/accounting queries in `bitmap.rs`, `fs.rs`, and `mod.rs` | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-BITMAP-21/10_creator_serial.md` | accepted `EXR-DIR-ENGINE-19` and free `fs.rs` write set | command-free lanes only | command-free production edit | returned and accepted for checker handoff | `.agents/components/EXR-BITMAP-21/01_designer_core.md` | `.agents/subagent-tasks/EXR-BITMAP-21/20260410-1245-creator-serial-packet.md` |
| `WS-BITMAP-21-CHECK-20260410` | `EXR-BITMAP-21` | Validate the new allocation-bitmap owner with local ktests, filtered `cargo osdk test` proof, and bounded checker-only debug support when needed | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-BITMAP-21/11_checker_serial.md` | returned `WS-BITMAP-21-CREATE-20260410`, checker lock, shared container health | command-free work outside the lock and future architect/designer lanes | serialized checker lane | passed | `.agents/components/EXR-BITMAP-21/03_designer_ktest.md` | `.agents/subagent-tasks/EXR-BITMAP-21/20260410-1335-checker-serial-packet.md` |
| `WS-BITMAP-21-REVIEW-20260410` | `EXR-BITMAP-21` | Review the landed bitmap-owner boundary after successful checker evidence, focusing on owner discipline, maintainability, and residual local risks | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-BITMAP-21/30_reviewer_report.md` | passing `WS-BITMAP-21-CHECK-20260410` | command-free lanes only | command-free review | passed with no production edits | `.agents/components/EXR-BITMAP-21/11_checker_serial.md` | `.agents/subagent-tasks/EXR-BITMAP-21/20260410-1455-reviewer-packet.md` |
| `WS-FS-OPEN-22-CREATE-20260410` | `EXR-FS-OPEN-22` | Implement the `ExfatFs` owner-side mount/open sequencing that installs prerequisites, publishes the canonical root inode, and removes the indefinite `root_inode()` seam | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-FS-OPEN-22/10_creator_serial.md` | accepted `EXR-BITMAP-21`, accepted `EXR-INODE-CACHE-18`, free `fs.rs` write set | command-free planning lanes only | command-free production edit | returned partially; repair needed | `.agents/components/EXR-FS-OPEN-22/01_designer_core.md` | `.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1510-creator-serial-packet.md` |
| `WS-FS-OPEN-22-REPAIR-20260410` | `EXR-FS-OPEN-22` | Repair the partial 22 creator return by landing the actual mount/open owner method and prerequisite ordering instead of only removing the old seam | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md` | partial `WS-FS-OPEN-22-CREATE-20260410`, free `fs.rs` write set remains reserved to this row | command-free planning lanes only | command-free production edit | returned and accepted for checker handoff | `.agents/components/EXR-FS-OPEN-22/10_creator_serial.md` | `.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1535-creator-repair-packet.md` |
| `WS-FS-OPEN-22-CHECK-20260410` | `EXR-FS-OPEN-22` | Validate the repaired mount/open sequence with local ktests, filtered `cargo osdk test` proof, and bounded checker-only debug support when needed | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-FS-OPEN-22/11_checker_serial.md` | accepted `WS-FS-OPEN-22-REPAIR-20260410`, checker lock, shared container health | command-free work outside the lock and artifact-only planning lanes | serialized checker lane | dispatched; artifact not yet recorded | `.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md` | `.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1605-checker-serial-packet.md` |
| `WS-DIR-OPS-23-ARCH-20260410` | `EXR-DIR-OPS-23` | Name the stable `ExfatInode` directory-lookup and `readdir_at` owner boundary that consumes the published root and `DirectoryEngine` without widening into mutation | `.agents/components/EXR-DIR-OPS-23/00_architect.md` | accepted `EXR-DIR-ENGINE-19`, accepted `EXR-UPCASE-20`, specified `EXR-FS-OPEN-22` boundary | the single creator round because the write set is artifact-only | command-free planning | returned and accepted | `.agents/components/WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1510-architect-packet.md` |
| `WS-DIR-OPS-23-DESIGN-20260410` | `EXR-DIR-OPS-23` | Turn the accepted inode-owned read-only directory boundary into a creator-ready `lookup` / `readdir_at` specification without widening into mutation or mount sequencing | `.agents/components/EXR-DIR-OPS-23/01_designer_core.md`, `.agents/components/EXR-DIR-OPS-23/02_designer_async.md`, `.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md` | accepted `WS-DIR-OPS-23-ARCH-20260410` | the single creator round because the write set is artifact-only | command-free planning | returned and accepted | `.agents/components/EXR-DIR-OPS-23/00_architect.md` | `.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1545-designer-packet.md` |
| `WS-FILE-MAP-24-ARCH-20260410` | `EXR-FILE-MAP-24` | Name the stable `ExfatInode` file-mapping boundary for logical-to-physical read-path translation without widening into data I/O, page-cache, or write-side growth semantics | `.agents/components/EXR-FILE-MAP-24/00_architect.md` | accepted `EXR-CHAIN-03B`, accepted `EXR-INODE-CORE-17`, specified `EXR-DIR-OPS-23` read-only path | the checker lane because the write set is artifact-only | command-free planning | returned and accepted | `.agents/components/WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-FILE-MAP-24/20260410-1620-architect-packet.md` |

Runtime lane assignment:

- `WS-BITMAP-21-CREATE-20260410`: returned from worker subagent `Zeno` (`019d764f-8598-7632-9202-cc1dea88a309`); creator artifact accepted locally and ready for checker handoff.
- `WS-BITMAP-21-CHECK-20260410`: returned from worker subagent `Zeno` (`019d764f-8598-7632-9202-cc1dea88a309`) with a passing checker artifact and no lingering temporary debug edits.
- `WS-BITMAP-21-REVIEW-20260410`: returned from worker subagent `Zeno` with no findings and no production edits; the row is accepted.
- `WS-FS-OPEN-22-CREATE-20260410`: returned from worker subagent `Zeno`, but the result only removed the old seam and added a root-publication regression; it did not yet land the actual mount/open sequencing method or prerequisite ordering from the designer spec.
- `WS-FS-OPEN-22-REPAIR-20260410`: returned from worker subagent `Zeno` with an owner-local `open_root_inode(&Arc<Self>)` path, mount-open serialization, root-directory prerequisite discovery through `DirectoryEngine`, upcase/bitmap installation ordering, and canonical root publication; the repair is accepted locally and is now checker-ready.
- `EXR-FS-OPEN-22` pre-research completed: architect and designer artifacts returned from worker `Sagan` (`019d764f-85de-7ad3-9bed-e7054b4bec2b`), which has since been resumed for `EXR-DIR-OPS-23` architect pre-research.
- `WS-DIR-OPS-23-ARCH-20260410`: returned from worker subagent `Sagan` (`019d764f-85de-7ad3-9bed-e7054b4bec2b`) and was accepted locally; the same worker is now assigned the 23 designer pass.
- `WS-DIR-OPS-23-DESIGN-20260410`: returned from worker subagent `Sagan` with a clean split designer set; the row is now specified and the same worker is reassigned to `EXR-FILE-MAP-24` architect pre-research.
- `WS-FILE-MAP-24-ARCH-20260410`: returned from worker subagent `Sagan` with a clean architect artifact; the row is now architected and the worker has been closed to keep the paused state lean.

## Recent Decisions

- This loop continues to use `$exfat-main-agent` for scheduler work and `$exfat-subagent-workflow` for delegated execution.
- The 16/17 rows were accepted without a final checker rerun because both reviewer artifacts reported no production edits.
- The 20/21 rows were advanced to `Specified` after their designer artifact sets returned.
- Only one creator round was opened in the preceding loop, for 18/19; the current loop is the follow-on checker pair for those same rows.
- Both checker packets explicitly permit local ktest additions, temporary checker-only debug output when needed, and debug-oriented `cargo osdk test` reruns, but only inside the packet write sets.
- The checker findings crossed cleanly: 18's checker was blocked by 19-side compile issues that 19's checker subsequently cleared, while 19's checker was then blocked by a local `fs.rs` move/borrow error. That makes a narrow 18-only repair the right next creator round.
- The narrow 18-only repair worked: both retry checker passes now have executable evidence, so 18/19 were advanced into reviewer wave instead of opening another creator pass.
- The reviewer wave split cleanly: 18 needed no edits and was accepted, while 19 made one narrow semantic hardening edit and therefore moved into a final checker instead of direct acceptance.
- The 19 final checker passed and backed the reviewer hardening with a new local regression, so 19 is now accepted.
- The next safe single creator round is `EXR-UPCASE-20`; `EXR-BITMAP-21` also needs `fs.rs` owner wiring and should wait until 20 closes.
- The first 20 creator delegation mis-executed and wrote an invalid reviewer artifact. That artifact was deleted, the stale worker was closed, and the creator round was reissued to a fresh worker, which then landed the correct `10_creator_serial.md`.
- The 20 checker passed with local ktests after one fixture repair, and the 20 reviewer returned with no production edits, so 20 is now accepted without a final checker rerun.
- With `fs.rs` free again, `EXR-BITMAP-21` becomes the next obvious single creator-round candidate.
- This loop adopts `EXR-BITMAP-21` as the only creator round and uses the remaining command-free budget on `EXR-FS-OPEN-22` architect+designer pre-research.
- The returned 21 creator diff stays inside the intended owner boundary: a new `bitmap.rs` module carries one immutable validated snapshot, while `fs.rs` only publishes and queries that state.
- The 21 checker added the required local bitmap regressions, fell back from one unusable exact suffix to a passing `bitmap::tests` module run, and still produced acceptable coverage proof.
- The 21 reviewer returned with no findings and no production edits, so acceptance does not require a final checker rerun.
- With bitmap accepted, the next safe single creator round is `EXR-FS-OPEN-22`; the remaining command-free budget shifts to `EXR-DIR-OPS-23` architect pre-research.
- The first 22 creator return is not checker-ready: it removed the indefinite `root_inode()` seam and added a local root-publication regression, but it did not implement the actual designer-required mount/open sequencing or prerequisite-order consumption path. That means the right next step is a narrow delegated creator repair, not a checker dispatch.
- The returned 22 repair pass closes that gap: the owner-local `open_root_inode(&Arc<Self>)` method now sequences `DirectoryEngine` discovery, upcase installation, bitmap installation, and canonical root publication in `fs.rs`, so checker can now validate behavior instead of completing missing implementation.
- The returned 23 architect artifact is good enough to advance the row without scheduler-side repair: it keeps `lookup` and `readdir_at` on `ExfatInode`, treats `DirectoryEngine` and `UpcaseTable` as consumed owners, and cleanly rejects a separate lookup-service boundary.
- The returned 23 designer set is also clean: it keeps directory ops read-only, keeps `lookup` / `readdir_at` on `ExfatInode`, and makes checker obligations local to `inode.rs`, so the row can advance to `Specified` without redesign.
- The returned 24 architect artifact is also clean: it keeps logical-to-physical mapping as `ExfatInode`-private read-path helpers, consumes `ExfatChain` and inode-owned size facts as existing boundaries, and does not widen into data I/O or page-cache ownership.
- The 2026-04-11 local 22 checker repair established that the old `11_checker_serial.md` result was not just a harness artifact: `qemu-serial.log` showed real failures in root-directory metadata handling and in the mount-ready fixture. Those were repaired narrowly, the retry checker passed, and the row is now accepted.

## Wave Record

- Scheduling or planning changes made in this wave:
  - closed the 16/17 follow-up loop by accepting both rows
  - advanced 20/21 into specified state
  - launched the next checker loop for 18/19
  - after both checker reports returned, opened a narrow 18-only repair creator round
  - after the repair landed, relaunched both checker passes and then opened the 18/19 reviewer wave
  - after reviewer return, accepted 18 and launched a final checker for 19
  - after the final checker returned, accepted 19 and launched the next single creator round for 20
  - after the first 20 creator mis-delegated, repaired delegation by deleting the invalid artifact, closing the stale worker, and reissuing the same creator packet to a fresh worker
  - launched the 20 checker lane, then launched the 20 reviewer lane after the checker passed
  - accepted 20 after the reviewer returned with no production edits
  - launched the 21 creator round
  - launched 22 architect pre-research in parallel
  - advanced 22 from `Planned` to `Architected` to `Specified` after the architect and designer artifacts returned
  - accepted the 21 creator return locally and prepared the 21 checker packet
  - dispatched the 21 checker lane to the same worker after creator acceptance
  - accepted the 21 checker return locally and prepared the 21 reviewer packet
  - accepted 21 after the reviewer returned with no production edits
  - launched the next single creator round for 22
  - launched 23 architect pre-research in parallel
  - rejected the first 22 creator return as partial and prepared a narrow creator-repair packet instead of misusing checker as a spec-completion lane
  - accepted the returned 23 architect artifact and launched the 23 designer pass
  - accepted the returned 22 repair artifact and prepared the 22 checker packet
  - accepted the returned 23 designer artifacts and launched 24 architect pre-research
  - accepted the returned 24 architect artifact and intentionally stopped there to preserve quota for the next loop
  - resumed the 22 checker locally on 2026-04-11, diagnosed the earlier failure chain through `qemu-serial.log`, landed a narrow creator repair, reran the checker, and accepted the row after a clean local reviewer pass
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - `EXR-FS-CORE-16` accepted
  - `EXR-INODE-CORE-17` accepted
  - `EXR-UPCASE-20` marked specified
  - `EXR-BITMAP-21` marked specified
  - `EXR-INODE-CACHE-18` checker returned blocked by cross-component build failures after adding local ktests
  - `EXR-DIR-ENGINE-19` checker returned blocked by a remaining local `fs.rs` compile error after adding local ktests
  - `EXR-INODE-CACHE-18` repair creator landed
  - `EXR-INODE-CACHE-18` retry checker passed
  - `EXR-DIR-ENGINE-19` retry checker passed
  - `EXR-INODE-CACHE-18` reviewer returned with no production edits and the row was accepted
  - `EXR-DIR-ENGINE-19` reviewer returned with a narrow hardening edit
  - `EXR-DIR-ENGINE-19` final checker passed and the row was accepted
  - `EXR-UPCASE-20` creator landed correctly after one repaired delegation attempt
  - `EXR-UPCASE-20` serial checker passed with local ktests
  - `EXR-UPCASE-20` reviewer returned with no production edits and the row was accepted
  - `EXR-BITMAP-21` creator returned with the intended `bitmap.rs`/`fs.rs`/`mod.rs` write set and no boundary drift
  - `EXR-BITMAP-21` checker passed with local bitmap regressions and no lingering temporary debug edits
  - `EXR-BITMAP-21` reviewer returned with no findings and the row was accepted
  - `EXR-FS-OPEN-22` creator returned partially and needs a repair creator pass before checker
  - `EXR-FS-OPEN-22` creator repair returned with the designer-required sequencing path and is ready for checker
  - `EXR-FS-OPEN-22` local checker retry proved the remaining failures were real prerequisite-discovery / fixture issues, the narrow repair landed, the retry checker passed, and the row was accepted
  - `EXR-DIR-OPS-23` architect returned and the row became architected
  - `EXR-DIR-OPS-23` designer returned and the row became specified
  - `EXR-FILE-MAP-24` architect returned and the row became architected
  - `EXR-FS-OPEN-22` architect returned and the row became architected
  - `EXR-FS-OPEN-22` designer returned and the row became specified
- Protocol, template, or packet-shaping changes made in this wave:
  - none beyond continued use of the skill-aware packet format introduced earlier in the day
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - completed reviewer and designer lanes from the prior loop are no longer active and are omitted from the live work-slice matrix

## Open Risks And Assumptions

- `EXR-DIR-OPS-23` is now the next creator candidate; it should stay read-only and inode-owned, without reopening mount/open or widening into namespace mutation.
- `EXR-FILE-MAP-24` must stay artifact-only until a creator round is explicitly scheduled, and its mapping boundary must remain separate from data I/O, page-cache, and write-side growth semantics.
- The 22 retry depended on reading `qemu-serial.log` to distinguish real test failures from truncated console output. Future checker loops should keep doing that whenever the terminal stream cuts off mid-line.
- Current filtered-test evidence may still require suffix-based proof instead of friendly runner output; preserve that discipline in later checker artifacts.

## Recommended Next Actions

1. Open the next single creator round on `EXR-DIR-OPS-23`.
2. In parallel, launch the `EXR-FILE-MAP-24` designer lane as an artifact-only pass.
3. Keep future checker diagnostics ready to inspect `/root/asterinas/qemu-serial.log` whenever the terminal stream truncates before the actual panic or test summary.

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff note.
- Treat `EXR-FS-OPEN-22` as accepted; the authoritative passing artifacts are `13_creator_serial_repair.md`, `14_checker_serial_retry.md`, and `30_reviewer_report.md`.
- Resume from the accepted 23 designer set plus the accepted 24 architect artifact, not from the older 22 checker-dispatch checkpoint.
- Read `PROTOCOL.md` when protocol maintenance or an explicit scheduler-rule question is in scope.
