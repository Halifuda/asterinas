<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff - 2026-08-10 Upstream Rebase + Post-Rebase API Repair

**Status:** `RECORD` — rebase + compile repair complete. Detailed change
record lives in the separate file
`../20260810-upstream-api-repair_record.md` (this handoff keeps only the
summary and pointers).

## Summary

- `codex/overlayfs-refactor` rebased onto `upstream/main` `94a8f624d`
  (2026-08-10); all 29 local commits preserved; local `main` fast-forwarded
  to upstream. Backup tag `backup/ovfs-pre-upstream-rebase-20260810` ->
  `cf8547536`.
- Rebase conflict at commit 7/29 (exFAT refactor) resolved in favor of the
  local refactored exFAT (user decision). Final exFAT tree identical to the
  pre-rebase tip.
- Post-rebase compile repair `e16075a72` adapts overlayfs/exfat to upstream
  VFS API changes: `Inode::metadata() -> Result`, `page_cache() ->
  Option<Arc<Vmo>>`, `PageCache` no longer `Clone` / `resize(&mut)` (exFAT
  uses `Once<Option<Mutex<PageCache>>>`), `FsType::Key` +
  `create(&mut FsCreationCtx)`, `Vmo::commit_on(.., VmoMapMode)`.
- Validation: `cargo check -p aster-kernel --target x86_64-unknown-none`
  exit 0 (only pre-existing `MountPolicy::uuid_mode` warning); clippy fails
  only on that same documented pre-existing warning.

## Next actions

- Full kernel build `make kernel` recommended before the next overlayfs pass.
- Do not force-push `codex/overlayfs-refactor` without explicit user request.

## Open issue — overlay/029 regression (claim/remount EBUSY)

Pointer: full investigation in
`../components/wave7-xfstests-sequencing/20260810_api_repair_rerun_checker.md`
§7–§11; evidence under
`../components/wave7-xfstests-sequencing/run_evidence/overlay029/` (reruns +
probe runs `20260810_probe_run{1,2}` / `20260810_probe_cycle{1,2}`).

Status: **diagnosed, NOT fixed** (awaiting user instruction).

- 20-case regression rerun after the API repair: 19 PASS / overlay/029 FAIL
  (mount EBUSY at the outer `_scratch_mount`). Refreshed-image rerun
  reproduced identically. No pollution (all 5 cases after 029 passed).
- Root cause (probe-confirmed, two probe rounds): the EBUSY comes from
  `OverlayInuseSlot::try_claim` on the upper/workdir inodes. The real
  mechanism is an internal reference chain under nested mounts: overlay/029's
  nested overlays resolve their lowerdirs inside the scratch overlay and
  store `Path` objects holding strong `Arc<Mount>` to the scratch overlay's
  Mount (`OverlayLayer.root_path` + `OverlayInode::facts.real_path`). Those
  references survive the nested teardown (scratch Mount strong count
  4→6→11), so the scratch `OverlayFs` is never dropped at unmount and its
  `UpperWorkdirClaim` is never released; the `./check` post-test remount of
  the same upper/workdir then fails the claim with EBUSY.
- Claim scope itself is correct (upper/workdir only; lower is not
  claim-checked; the stacked-lower/d_real body of 029 runs fine).
- Fix directions (not implemented): wire claim release to unmount (free the
  slots even if the fs Arc lives), or break the reference chain (stored
  `Path`s should not hold strong Mount refs / clean them at teardown).

## Scheduled — Designer dispatch for overlay/029 fix-scheme selection (2026-08-10)

User instruction: dispatch a Designer to decide the fix scheme for the
nested-mount claim-lifetime / self-reference ring; candidate directions are
umount-time claim release, Weak-mount carriers (Weak<Path>), and umount 手动
解决 (manual teardown cleanup) — the Designer must analyze safety in depth
before freezing one.

- Task ID: `task_designer_nested_mount_claim_lifetime_20260810`
- Packet: `../subagent-tasks/nested_mount_claim_lifetime_design/task_designer_nested_mount_claim_lifetime_20260810_dispatch.md`
- Component group: `nested_mount_claim_lifetime_design`
- Covered micros: `P1-35` (direct), `P0-02`, `P0-16` (contributing carriers)
- Write-set: `components/nested_mount_claim_lifetime_design/nested_mount_claim_lifetime_designer_{spec,validation}.md`
- Gate: main-agent structural acceptance of the frozen scheme + exact Rust
  surface; any VFS-touching item must be enumerated and flagged for explicit
  user/main-agent authorization before any Creator dispatch.
- Next action: user posts the V2 User Dispatch Turn verbatim; main agent
  spawns immediately (`task_name=<task_id>`, `fork_turns="1"`).

## ACCEPTED — Designer `task_designer_nested_mount_claim_lifetime_20260810` (2026-08-10)

Structural acceptance passed (artifacts reviewed against the packet
acceptance boundary; key file:line claims spot-verified against the tree).

- Artifacts:
  `components/nested_mount_claim_lifetime_design/nested_mount_claim_lifetime_designer_spec.md`
  `components/nested_mount_claim_lifetime_design/nested_mount_claim_lifetime_designer_validation.md`
- Frozen scheme: **B1-local — overlayfs-local weak-mount carriers**. New
  `RealPath` carrier (`Weak<Mount>` + `Arc<Dentry>` + `Arc<dyn Inode>`) in
  `mount/layers.rs`; `OverlayLayer.root_path` (P0-02) and
  `RealObject.real_path` (P0-16) become `RealPath`; per-use
  `RealPath::upgrade() -> Result<Path>` (`EIO` on dead anchor). Claim
  mechanism untouched: `UpperWorkdirClaim` still released by guard `Drop` on
  the final `OverlayFs` `Drop` (P1-35). Zero VFS edits; `begin_shutdown`
  stays dead_code/not wired.
- Rejected: Direction A (umount-time claim release — breaks meso-01 Hazard 4
  / releases under live consumers / Linux `ovl_free_fs` parity), B2 (manual
  teardown cleanup — cannot reach surviving nested carriers; facts
  immutable), B1-VFS (`Path { mount: Weak<Mount> }` — VFS-wide blast radius,
  flagged requires authorization, superseded by B1-local).
- Expected Creator change set: 9 files (`mount/layers.rs`,
  `projection/entry.rs`, `projection/inode.rs`, `dir/{mod,link,remove,rename,
  create}.rs`, `copyup/promote.rs`); 16-row call-site disposition table in
  spec §5; entity census in §6.1.
- Validation contract: step (i) single-case `overlay/029` fresh 8 GiB whole-
  case PASS, zero EBUSY/warn/oops; step (ii) full 20-case 20260810 matrix all
  PASS / 0 NOTRUN / 0 HANG / no pollution; unmount→remount invariant is a
  mandatory Checker assertion; coverage limitation recorded (EBUSY live-
  conflict and dead-mount EIO paths are no-upstream-coverage).
- Next action (awaiting user): confirm the B1-local scheme, then main agent
  slices + dispatches a Creator pass (command-free) against the frozen
  surface, followed by Checker validation per the contract.

## ACCEPTED — pass_45_nested_mount_claim_lifetime (2026-08-10)

Creator + Checker gate complete. The overlay/029 nested-mount claim-lifetime
bug is fixed by the user-confirmed **B1-local** scheme.

- Creator `task_creator_nested_mount_claim_lifetime_20260810` (agent Hooke):
  implemented the frozen 会签 surface (`RealPath` weak-mount carrier,
  `OverlayLayer.root_path`/`RealObject.real_path` type deltas, 9 mechanical
  adaptations; one incidental `mount/mod.rs` re-export seam mirroring
  `XinoMode`); report
  `components/nested_mount_claim_lifetime_design/pass_45_nested_mount_claim_lifetime_creator.md`.
  Main-agent structural diff acceptance PASSED; container
  `cargo check -p aster-kernel --target x86_64-unknown-none` PASSED (0
  errors, only the pre-existing `uuid_mode` warning). Committed `c92c21b67`
  (12 files, +251/-49).
- Checker `task_checker_nested_mount_claim_lifetime_20260810` (agent
  Kierkegaard), **scoped to `overlay/029` single-case only** (user-directed;
  full 20-case regression deferred until after wave8): whole-case **PASS** on
  fresh 8 GiB images — `Ran: overlay/029` / `Passed all 1 tests` / `All
  conformance tests passed.`; zero EBUSY / `already mounted or mount point
  busy.` / `try_claim` trace; zero kernel warn/oops/panic; unmount→remount
  invariant satisfied (the exact inverse of the pre-fix EBUSY signature).
  Receipt
  `components/nested_mount_claim_lifetime_design/pass_45_nested_mount_claim_lifetime_checker.md`;
  evidence
  `components/nested_mount_claim_lifetime_design/run_evidence/overlay029_pass45_20260810/`.
- Deferred: the 20-case regression matrix (overlay/002..077) runs after
  wave8 per user direction.
