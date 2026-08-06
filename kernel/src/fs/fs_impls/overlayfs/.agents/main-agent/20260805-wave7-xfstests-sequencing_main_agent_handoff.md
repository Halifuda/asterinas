<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-05 Wave7 xfstests Sequencing

**Date / Time:** 2026-08-05
**Status:** `Planned / Not Started. This handoff records the ordered
current-scope xfstests obligation. The user-directed mechanical deferred-expect
cleanup recorded in Section 4 is complete, but it neither starts Wave7 nor
authorizes a Wave7 packet, Checker command, xfstests result, or
`legacy_fs.rs` deletion.`

## 1. Global State Pointer

- **Current Active Wave / Pass:** Wave7 xfstests sequencing
  (`wave7_xfstests_sequencing_20260805`), planned only.
- **Predecessor:** Wave6 documentation review is closed. Its static chain
  (workspace Clippy, `cargo fmt --check`, and `make check`) passed; this does
  not constitute runtime xfstests evidence or final acceptance.
- **Blueprint Updates Made:** No. This handoff is the user-requested planning
  record only; `SYSTEM_BLUEPRINT.md` and `PASS_SLICING.md` retain their
  accepted Wave6 state until Wave7 is expressly started.
- **Scope source:** Stage D makes all P0/P1 and `P2-01 xino` mandatory, and
  the later accepted scope amendment also includes `P2-11 UUID modes`.
  The six accepted Designer validation contracts remain the external-evidence
  source. xfstests is many-to-many evidence, not a one-test-per-micro claim.

## 2. Ordered Current-Scope xfstests Obligation

Run the following cases one at a time, in this order, once Wave7 is explicitly
started and a Checker packet has authorized the runtime lane. Before attributing
any result, the Checker must confirm the actual case setup and asserted theme
from the upstream suite source, preserve the result file and guest log, and
record `PASS`, `FAIL`, or `NOTRUN` per case. A failing case follows the normal
Checker evidence and repair-loop protocol; this table does not authorize a
repair or rerun.

| Order | Case | Current-scope purpose |
| ---: | :--- | :--- |
| 1 | `overlay/001` | Minimal mount, option validation, and root/stat baseline. |
| 2 | `overlay/021` | Writable mount with upper/workdir setup. |
| 3 | `overlay/035` | Read-only mount without `upperdir`. |
| 4 | `overlay/002` | Basic upper-first lookup and merged-directory root path. |
| 5 | `overlay/003` | Basic char-device whiteout observation. |
| 6 | `overlay/004` | Lookup handling of opaque and whiteout state. |
| 7 | `overlay/005` | Merged-directory lookup and readdir dedup baseline. |
| 8 | `overlay/007` | Merged readdir and `d_ino` baseline. |
| 9 | `overlay/019` | Stat/readdir identity consistency. |
| 10 | `overlay/013` | First writable copy-up plus upper-only create. |
| 11 | `overlay/027` | Copy-up followed by upper creation. |
| 12 | `overlay/009` | Basic data/xattr copy-up. |
| 13 | `overlay/014` | Multi-lower copy-up and eligible xattrs. |
| 14 | `overlay/029` | Lower-backed file access and real-file delegation. |
| 15 | `overlay/039` | Lower-file mmap delegation and divergence behavior. |
| 16 | `overlay/040` | fsync/fdatasync delegation after file I/O. |
| 17 | `overlay/023` | Workdir temporary cleanup after the ordinary copy-up path. |
| 18 | `overlay/025` | setattr-driven copy-up and metadata forwarding. |
| 19 | `overlay/078` | Permission/default-permissions mount-option behavior and packaged UUID-mode observation. |
| 20 | `overlay/008` | Create over whiteout and object-dispatch behavior. |
| 21 | `overlay/015` | SGID/permission behavior over a whiteout. |
| 22 | `overlay/016` | SGID/permission behavior for create-family operations. |
| 23 | `overlay/010` | rmdir and lower-directory hiding. |
| 24 | `overlay/011` | Unlink/hardlink-over-whiteout behavior. |
| 25 | `overlay/012` | Stale-dentry recovery after unlink. |
| 26 | `overlay/020` | Create/unlink publication and cache invalidation. |
| 27 | `overlay/026` | Symlink copy-up and get-link delegation. |
| 28 | `overlay/024` | Origin-record copy-up and identity stability across remount. |
| 29 | `overlay/031` | Whiteout invisibility and persistence across remount. |
| 30 | `overlay/032` | Basic same- and cross-directory rename semantics. |
| 31 | `overlay/033` | Rename requiring copy-up. |
| 32 | `overlay/034` | Rename with whiteout publication and no-side-effect `EXDEV` path. |
| 33 | `overlay/006` | Lower-backed mutation trigger and resulting whiteout behavior. |
| 34 | `overlay/037` | Copy-up-induced readdir-cache invalidation. |
| 35 | `overlay/077` | Readdir cache invalidation and stale-entry convergence. |
| 36 | `overlay/038` | `P2-01` xino same-filesystem `d_ino` consistency. |
| 37 | `overlay/041` | `P2-01` xino non-same-filesystem `d_ino` consistency. |
| 38 | `overlay/042` | `P2-01` xino `st_ino` consistency. |

The order is intentional: mount and read-only fundamentals precede projection
and readdir; ordinary single-object copy-up and file views precede metadata and
workdir cleanup; ordinary namespace mutations precede remount and rename
transactions; cache-invalidation paths follow their mutation/copy-up causes;
xino follows stable lookup, readdir, copy-up, and identity behavior.

## 3. Explicit Scope Exclusions

- Do not schedule `overlay/017`, `043`, or `057`: they require deferred
  `P2-02 redirect_dir` behavior, even where they incidentally observe a basic
  stat or xino path.
- Do not schedule `overlay/018`, `028`, or `044` as required whole-case
  passes: their lower-hardlink/origin/nlink assertions need deferred
  association/index or `P2-07` behavior. The current basic contract permits
  only the non-index upper-authoritative link observation, not treating a
  whole mixed-scope case as passed.
- Do not schedule `overlay/030`, `075`, or `076` (deferred `P2-06`
  fileattr), `overlay/083`, `084`, or `109` (deferred userxattr/escaping or
  unavailable packaged lane), or any P3 index, NFS export, metacopy,
  data-only, fs-verity, trap, nested-overlay, or cleanup case.
- Do not schedule `overlay/067` or `068` as UUID tests. The mount validation
  contract calls them best-known UUID candidates, but the staged inventory
  assigns them P3 index/NFS behavior, while its dedicated `overlay/081` UUID
  row is absent from the packaged `full.list`. Until an explicitly authorized
  source-level reconciliation changes that mapping, `overlay/078` is the only
  packaged current-scope UUID observation in this order.

## 4. Thread Activity Log

- **User-directed mechanical deferred-expect cleanup (2026-08-06):** Removed
  the unused claim and UUID accessors from `mount/claims.rs` and
  `mount/policy.rs`: `InodeClaimGuard::token`,
  `UpperWorkdirClaim::{has_exclusive_claim,upper_inode,identity}`, and
  `MountPolicy::uuid_mode`; and removed the now-unreferenced
  `OverlayInuseSlot::is_claimed_by` query. `UpperWorkdirClaim` no longer
  duplicates the upper filesystem `Arc`; `selected_real_fs` now reads the
  canonical upper filesystem from `OverlayLayerStack`. This is ownership
  cleanup only: claim acquisition, release, lifecycle semantics, and
  upper/lower selection remain unchanged. The nine retained `dead_code`
  expectations are the two VFS
  unmount/shutdown seams, two VFS scoped-credential seams, and five VFS
  writer/freeze seams; each now carries a concrete TODO and VFS-specific
  reason. `cargo fmt --check` and `git diff --check` passed. The usual
  workspace Clippy command could not start because this host lacks
  `cargo-osdk`; a standalone `cargo check -p aster-kernel` with an isolated
  target directory instead failed in pre-existing host configuration before
  compiling the kernel crate, because OSDK-provided architecture dependencies
  were absent. No Wave7 runtime command ran.
- **Dispatches Sent:** None.
- **Commands Run:** None for Wave7.
- **Acceptance Outcomes:** None. This is a scheduling record, not an
  xfstests result or an integration acceptance.
- **Escalations / Deadlocks:** None. The UUID mapping discrepancy is recorded
  above as a pre-start reconciliation item, not a runtime failure.

## 5. Explicit Agent-Level Decisions

1. Wave7 remains unstarted. The next action is not implicit from this handoff.
2. Only complete xfstests cases whose intended current-scope behavior can be
   passed as a whole are ordered. Mixed cases with deferred required assertions
   are excluded rather than counted as partial success.
3. Creating this handoff did not itself authorize test, harness,
   `legacy_fs.rs`, production, VFS, Designer, Creator, or Reviewer work. The
   separately user-directed mechanical cleanup is recorded in Section 4 and
   does not authorize Wave7 implementation or runtime work.
4. A future runtime packet must retain xfstests as the sole validation lane;
   no ktest or filesystem-local substitute is permitted.

## 6. Next Actions for the Next Thread (CRITICAL)

1. Wait for explicit user direction to start Wave7; do not dispatch a Checker
   or run any command beforehand.
2. On authorization, create one Checker-owned runtime packet that names the
   first case only, its preserved-evidence location, the applicable accepted
   Designer validation contracts, and the exact scope exclusions above.
3. Before running `overlay/078`, reconcile the packaged UUID test mapping
   against the current upstream suite source. Do not promote `overlay/067`,
   `068`, or unavailable `081` into Wave7 without a new scope decision.

## 7. Live File Discipline

- **This file is the live handoff for:** the planned Wave7 xfstests tenure.
- **Update rule:** Update this file in place for every Wave7 start decision,
  dispatch, result, repair routing, acceptance, rejection, or escalation.
- **Supersedes / Replaces:**
  `20260804-wave6-documentation-lint_main_agent_handoff.md`, closed / handed
  over on 2026-08-05.
