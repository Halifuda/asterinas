<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `river-anvil`
- Date: 2026-04-07 10:10 CST
- Covered hours: initial resume checkpoint
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: host workspace inspection only
- Status: resumed after `clear-forge`; Wave A serial checker retry and follow-on command-free architect lanes in progress

## Environment Summary

- Image or base environment: not revalidated in this resume pass
- Working path: `/home/halifuda/asterinas`
- Container name, if any: `codex-asterinas-dev` restarted and observed running on 2026-04-07 after the first serial checker attempt found it exited
- KVM status: retry checker observed `no-kvm`; filtered ktests ran under QEMU TCG fallback
- Validated commands:
  - read-only repository inspection under `.agents/`
  - read-only inspection of VFS trait carrier surfaces and legacy exFAT carrier references
  - `git status --short`
  - `date '+%Y-%m-%d %H:%M %Z'`
- Known environment blockers:
  - no runtime verification was attempted
  - current worktree has untracked `.codex`
  - `amper-ledger-20260402-1712-protocol-prior-tightening.md` is present as an untracked hand-restored protocol-history file and must not be treated as rollback debris

## Current Project State

- Current goal: resume main-agent control after the rollback and owner-first board reset without reopening ownerless staging work
- Current phase: Wave A serial checker retry is active; reviewer packets are prepared; follow-on `EXR-UPCASE-20` / `EXR-BITMAP-21` architect lanes launched
- Active or next component:
  - `EXR-FS-CORE-16` serial checker retry passed and reviewer packet is ready
  - `EXR-INODE-CORE-17` serial checker retry is active and reviewer packet is ready once the retry report lands
  - `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19` creator packets are prepared but intentionally not launched while `fs.rs`/`mod.rs` reviewer write sets may still be active
  - `EXR-UPCASE-20` and `EXR-BITMAP-21` architect lanes are active command-free follow-on work
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
  - `EXR-SBGEOM-15`
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
- Components in progress:
  - none
- Blocked components:
  - none formally blocked

## Active Work Slice Matrix

This is the scheduler-owned global view of currently adopted work slices.
Architect artifacts may recommend local candidate slices, but this matrix is the authoritative active plan.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FS-CORE-16-ARCH` | `EXR-FS-CORE-16` | Define the `ExfatFs` owner boundary, `FileSystem` skeleton obligations, `root_inode()` seam decision, and candidate creator slices | `.agents/components/EXR-FS-CORE-16/00_architect.md` | Accepted foundations through `EXR-SBGEOM-15`; board reset | `WS-INODE-CORE-17-ARCH` only as a command-free sibling with disjoint write set | command-free | completed; accepted | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1021-architect-packet.md` |
| `WS-INODE-CORE-17-ARCH` | `EXR-INODE-CORE-17` | Define the `ExfatInode` owner boundary, initial `Inode` core obligations, `ExfatFs` handshake assumptions, and candidate creator slices | `.agents/components/EXR-INODE-CORE-17/00_architect.md` | Accepted `EXR-FILESET-04B`, `EXR-CHAIN-03B`; `EXR-FS-CORE-16` owner contract assumptions | `WS-FS-CORE-16-ARCH` only as a command-free sibling with disjoint write set | command-free | completed; accepted | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1021-architect-packet.md` |
| `WS-FS-CORE-16-DESIGN` | `EXR-FS-CORE-16` | Specify the `ExfatFs` owner skeleton, `FileSystem` method scope, `root_inode()` temporary seam, and checker-owned test obligations | `.agents/components/EXR-FS-CORE-16/01_designer_core.md`, optional `02_designer_async.md`, `.agents/components/EXR-FS-CORE-16/03_designer_ktest.md` | Accepted `EXR-FS-CORE-16` architect artifact | `WS-INODE-CORE-17-DESIGN` only as a command-free sibling with disjoint write set | command-free | completed; accepted | `.agents/components/EXR-FS-CORE-16/00_architect.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1035-designer-packet.md` |
| `WS-INODE-CORE-17-DESIGN` | `EXR-INODE-CORE-17` | Specify the `ExfatInode` owner skeleton, metadata surface, explicit read/write seams, and checker-owned test obligations | `.agents/components/EXR-INODE-CORE-17/01_designer_core.md`, optional `02_designer_async.md`, `.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md` | Accepted `EXR-INODE-CORE-17` architect artifact | `WS-FS-CORE-16-DESIGN` only as a command-free sibling with disjoint write set | command-free | completed; accepted | `.agents/components/EXR-INODE-CORE-17/00_architect.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1035-designer-packet.md` |
| `WS-INODE-CACHE-18-ARCH` | `EXR-INODE-CACHE-18` | Define the `ExfatFs`-owned opened-inode table and validated `InodeKey` boundary without reviving standalone INOKEY drift | `.agents/components/EXR-INODE-CACHE-18/00_architect.md` | Accepted `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` architect artifacts | `WS-DIR-ENGINE-19-ARCH` and Wave A designer lanes with disjoint write sets | command-free | completed; accepted | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1040-architect-packet.md` |
| `WS-DIR-ENGINE-19-ARCH` | `EXR-DIR-ENGINE-19` | Define the `ExfatFs`-owned read-only `DirectoryEngine` record-stream boundary | `.agents/components/EXR-DIR-ENGINE-19/00_architect.md` | Accepted `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B` | `WS-INODE-CACHE-18-ARCH` and Wave A designer lanes with disjoint write sets | command-free | completed; accepted | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1040-architect-packet.md` |
| `WS-FS-CORE-16-CREATE` | `EXR-FS-CORE-16` | Implement `ExfatFs` owner skeleton and own shared `mod.rs` module declarations for Wave A | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-FS-CORE-16/10_creator_serial.md` | Accepted `EXR-FS-CORE-16` designer artifacts | `WS-INODE-CORE-17-CREATE` with disjoint write set; owns the shared `mod.rs` collision | command-free | completed | `.agents/components/EXR-FS-CORE-16/01_designer_core.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1043-creator-serial-packet.md` |
| `WS-INODE-CORE-17-CREATE` | `EXR-INODE-CORE-17` | Implement `ExfatInode` metadata carrier in `inode.rs` without cache/page-cache/data-path behavior | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-INODE-CORE-17/10_creator_serial.md` | Accepted `EXR-INODE-CORE-17` designer artifacts and sibling `ExfatFs` assumptions | `WS-FS-CORE-16-CREATE` with disjoint write set; must not edit `mod.rs` | command-free | completed | `.agents/components/EXR-INODE-CORE-17/01_designer_core.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1043-creator-serial-packet.md` |
| `WS-INODE-CACHE-18-DESIGN` | `EXR-INODE-CACHE-18` | Specify opened-inode table, `InodeKey`, root special case, and checker-owned tests | `.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`, optional `02_designer_async.md`, `.agents/components/EXR-INODE-CACHE-18/03_designer_ktest.md` | Accepted `EXR-INODE-CACHE-18` architect artifact | `WS-DIR-ENGINE-19-DESIGN` and Wave A creator lanes with disjoint write sets | command-free | completed; accepted | `.agents/components/EXR-INODE-CACHE-18/00_architect.md` | `.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1048-designer-packet.md` |
| `WS-DIR-ENGINE-19-DESIGN` | `EXR-DIR-ENGINE-19` | Specify read-only `DirectoryEngine` record-stream contract and checker-owned tests | `.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`, optional `02_designer_async.md`, `.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md` | Accepted `EXR-DIR-ENGINE-19` architect artifact | `WS-INODE-CACHE-18-DESIGN` and Wave A creator lanes with disjoint write sets | command-free | completed; accepted | `.agents/components/EXR-DIR-ENGINE-19/00_architect.md` | `.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1048-designer-packet.md` |
| `WS-FS-CORE-16-CHECK` | `EXR-FS-CORE-16` | Validate `ExfatFs` owner skeleton and temporary seams, with minimal in-scope fixes if required | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-FS-CORE-16/11_checker_serial.md` | Completed `EXR-FS-CORE-16` creator pass | Command-free lanes only; command execution serialized by checker lock | runtime/test-producing | launched | `.agents/components/EXR-FS-CORE-16/10_creator_serial.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1052-checker-serial-packet.md` |
| `WS-INODE-CORE-17-CHECK` | `EXR-INODE-CORE-17` | Validate `ExfatInode` metadata carrier and temporary seams, with minimal in-scope fixes if required | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-INODE-CORE-17/11_checker_serial.md` | Completed `EXR-INODE-CORE-17` creator pass | Command-free lanes only; command execution serialized by checker lock | runtime/test-producing | completed with environment blocker; retry launched | `.agents/components/EXR-INODE-CORE-17/10_creator_serial.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1052-checker-serial-packet.md` |
| `WS-FS-CORE-16-CHECK-RETRY` | `EXR-FS-CORE-16` | Rerun filtered executable verification for `ExfatFs` after restarting `codex-asterinas-dev` | `.agents/components/EXR-FS-CORE-16/12_checker_serial_retry.md` plus in-scope `fs.rs`/`mod.rs` repair if required | Environment-blocked `WS-FS-CORE-16-CHECK` | Runtime checker lanes through checker lock; command-free architect/reviewer packet prep | runtime/test-producing | completed; filtered ktests passed under TCG fallback | `.agents/components/EXR-FS-CORE-16/11_checker_serial.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1100-checker-serial-retry-packet.md` |
| `WS-INODE-CORE-17-CHECK-RETRY` | `EXR-INODE-CORE-17` | Rerun filtered executable verification for `ExfatInode` after restarting `codex-asterinas-dev` | `.agents/components/EXR-INODE-CORE-17/12_checker_serial_retry.md` plus in-scope `inode.rs` repair if required | Environment-blocked `WS-INODE-CORE-17-CHECK` | Runtime checker lanes through checker lock; command-free architect/reviewer packet prep | runtime/test-producing | launched | `.agents/components/EXR-INODE-CORE-17/11_checker_serial.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1100-checker-serial-retry-packet.md` |
| `WS-FS-CORE-16-REVIEW` | `EXR-FS-CORE-16` | Review the filesystem owner skeleton after serial checker retry | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-FS-CORE-16/30_reviewer_report.md` | Passed `WS-FS-CORE-16-CHECK-RETRY` | `WS-INODE-CORE-17-REVIEW` if sibling stays out of `fs.rs`/`mod.rs` | command-free | packet prepared; not yet launched | `.agents/components/EXR-FS-CORE-16/12_checker_serial_retry.md` | `.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1105-reviewer-packet.md` |
| `WS-INODE-CORE-17-REVIEW` | `EXR-INODE-CORE-17` | Review the inode metadata carrier after serial checker retry | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `.agents/components/EXR-INODE-CORE-17/30_reviewer_report.md` | `WS-INODE-CORE-17-CHECK-RETRY` | `WS-FS-CORE-16-REVIEW` if sibling stays out of `inode.rs` | command-free | packet prepared; waiting on retry report | `.agents/components/EXR-INODE-CORE-17/12_checker_serial_retry.md` | `.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1105-reviewer-packet.md` |
| `WS-INODE-CACHE-18-CREATE` | `EXR-INODE-CACHE-18` | Implement the `ExfatFs` opened-inode table and owner-private `InodeKey` boundary | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `.agents/components/EXR-INODE-CACHE-18/10_creator_serial.md` | Completed 16/17 reviewer passes | Avoid overlap with any `fs.rs` writer | command-free production edit | packet prepared; deferred for write-set safety | `.agents/components/EXR-INODE-CACHE-18/01_designer_core.md` | `.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1105-creator-serial-packet.md` |
| `WS-DIR-ENGINE-19-CREATE` | `EXR-DIR-ENGINE-19` | Implement the read-only `DirectoryEngine` record-stream service | `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md` | Completed 16 reviewer pass if `mod.rs` remains a collision point | Avoid overlap with any `mod.rs` writer | command-free production edit | packet prepared; deferred for write-set safety | `.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md` | `.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1105-creator-serial-packet.md` |
| `WS-UPCASE-20-ARCH` | `EXR-UPCASE-20` | Define the `ExfatFs`-owned upcase-table runtime state and name-folding/hash service boundary | `.agents/components/EXR-UPCASE-20/00_architect.md` | Accepted `EXR-DIR-ENGINE-19` architect/designer boundary | `WS-BITMAP-21-ARCH` and runtime checker lanes with disjoint write sets | command-free | launched | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-UPCASE-20/20260407-1110-architect-packet.md` |
| `WS-BITMAP-21-ARCH` | `EXR-BITMAP-21` | Define the `ExfatFs`-owned allocation-bitmap runtime state and read-only occupancy query boundary | `.agents/components/EXR-BITMAP-21/00_architect.md` | Accepted `EXR-DIR-ENGINE-19` architect/designer boundary | `WS-UPCASE-20-ARCH` and runtime checker lanes with disjoint write sets | command-free | launched | `WORKSPACE-ARCH-RESET/00_architect.md` | `.agents/subagent-tasks/EXR-BITMAP-21/20260407-1110-architect-packet.md` |

Runtime lane assignment:

- `WS-FS-CORE-16-ARCH`: delegated to worker subagent `Lorentz` (`019d65c1-f616-7c22-a02e-e70ef5166c59`).
- `WS-INODE-CORE-17-ARCH`: delegated to worker subagent `Newton` (`019d65c1-f63e-7c61-a767-3eb511807252`).
- `WS-FS-CORE-16-DESIGN`: delegated to worker subagent `Ptolemy` (`019d65ca-fa26-7ba3-8e4b-e42834cda4e6`).
- `WS-INODE-CORE-17-DESIGN`: delegated to worker subagent `Curie` (`019d65ca-fa70-7ae2-8a3b-358541bf22ad`).
- `WS-INODE-CACHE-18-ARCH`: delegated to worker subagent `Socrates` (`019d65cc-ac8d-7730-9d55-187578f0a77e`).
- `WS-DIR-ENGINE-19-ARCH`: delegated to worker subagent `McClintock` (`019d65cc-acb6-75b2-accd-374bfb5ab7fa`).
- `WS-FS-CORE-16-CREATE`: delegated to worker subagent `Cicero` (`019d65cf-4bf4-7840-b7fb-a0eb7947cb87`).
- `WS-INODE-CORE-17-CREATE`: delegated to worker subagent `Bernoulli` (`019d65cf-4c25-7ae1-ac51-53bdc320e755`).
- `WS-INODE-CACHE-18-DESIGN`: delegated to worker subagent `Boole` (`019d65d1-c796-7743-8b13-73946f6cf8a1`).
- `WS-DIR-ENGINE-19-DESIGN`: delegated to worker subagent `Parfit` (`019d65d1-c7c2-7a62-92f6-22e4b841de3d`).
- `WS-FS-CORE-16-CHECK`: delegated to worker subagent `Bohr` (`019d65d6-5d4f-7db3-99ab-9544b10ee674`).
- `WS-INODE-CORE-17-CHECK`: delegated to worker subagent `Copernicus` (`019d65d6-5d7f-7012-a87f-a6b8d178db4c`).
- `WS-FS-CORE-16-CHECK-RETRY`: delegated to worker subagent `James` (`019d65e1-737b-7811-8146-117eb7141498`).
- `WS-INODE-CORE-17-CHECK-RETRY`: delegated to worker subagent `Volta` (`019d65e1-98a4-7ae3-94cf-f262d964120a`).
- `WS-UPCASE-20-ARCH`: delegated to worker subagent `Euler` (`019d65e5-b900-7450-8b20-68cb52de6aed`).
- `WS-BITMAP-21-ARCH`: delegated to worker subagent `Erdos` (`019d65e5-d92c-74d0-9fd5-904995a77d7a`).

## Recent Decisions

- The rollback happened because the post-`EXR-SBGEOM-15` wave starting at `EXR-INOKEY-05A` drifted into ownerless staging surfaces, free helpers, and intermediate structs instead of converging on stable trait carriers and runtime owners.
- Future architect work must define functional units only after naming the final architectural owner and landing form. Work-slice convenience, dependency safety, or parallelism may shape packet cuts only after the owner boundary is justified.
- Future designer work must refine the architected owner-first unit. It must not silently promote an owner-internal slice into a standalone public module, and it must reject specs that are still too coarse for narrow creator passes.
- `COMPONENT_INDEX.md` was checked against `WORKSPACE-ARCH-RESET/00_architect.md`; the planned rows `EXR-FS-CORE-16` through `EXR-SYNC-31` match the proposal's owner, landing-form, and dependency shape in the checked areas.
- The next wave should treat `EXR-FS-CORE-16` as the first convergence point: define `ExfatFs` as the stable VFS `FileSystem` carrier and runtime-state root before trying to land `ExfatInode` cache or mount sequencing details.
- `EXR-FS-CORE-16` had one explicit architect-stage risk: the VFS `FileSystem::root_inode()` contract returns `Arc<dyn Inode>`, but `EXR-INODE-CORE-17` is a sibling unit. The accepted architect result resolves this by keeping `root_inode()` as an explicit temporary seam with a named exit plan into `EXR-FS-OPEN-22` after `EXR-INODE-CORE-17` lands. In the current architecture phase, `todo!` or `unimplemented!` may be acceptable when they are explicit, unreachable from the registered legacy filesystem, and recorded with an exit condition; what remains forbidden is using them to hide a fake root inode, ownerless staging type, or unclear long-lived boundary.
- Ordinary-subagent delegation in this wave must use the archived packets under `.agents/subagent-tasks/<component-id>/` and be accompanied by `COMMON_SUBAGENT.md` plus the matching role protocol file. `PROTOCOL.md` must remain main-agent-facing unless the delegated task is itself scheduler/protocol work.
- Wave A architect packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` are now archived and launched as command-free sibling lanes with disjoint artifact write sets.
- Wave A architect artifacts are accepted as mutually consistent. `EXR-FS-CORE-16` keeps `root_inode()` as an explicit temporary seam; `EXR-INODE-CORE-17` defines the real inode carrier without absorbing inode cache, page cache, directory ops, read/write, or namespace work. Future creator scheduling must serialize any shared `mod.rs` declaration edit instead of pretending the first production pass is fully file-parallel.
- Wave A designer packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` are archived and launched as command-free sibling lanes.
- Follow-on architect packets for `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19` are archived and launched while Wave A designer work runs, because their artifact write sets are disjoint and their dependencies are architecturally stable enough for command-free planning.
- Wave A creator packets are archived and launched. `EXR-FS-CORE-16` owns `mod.rs` declaration edits for both `fs` and `inode`; `EXR-INODE-CORE-17` must not edit `mod.rs`.
- Wave A designer artifacts are accepted and `COMPONENT_INDEX.md` advanced `EXR-FS-CORE-16` / `EXR-INODE-CORE-17` to `Specified`.
- Follow-on architect artifacts for `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19` are accepted and `COMPONENT_INDEX.md` advanced both rows to `Architected`.
- Follow-on designer packets for `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19` are archived and launched while Wave A creator work runs.
- Wave A creator artifacts returned for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`; checker packets are archived and launched with lock-guarded execution.
- Follow-on designer artifacts for `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19` are accepted and `COMPONENT_INDEX.md` advanced both rows to `Specified`.
- The first 16/17 serial checker pass was environment-blocked because `codex-asterinas-dev` was not running. The container was restarted and retry checker packets were archived and launched.
- `EXR-FS-CORE-16` retry checker passed all three filtered ktests. The checker observed no KVM and QEMU used TCG fallback.
- Reviewer packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`, plus creator packets for `EXR-INODE-CACHE-18` and `EXR-DIR-ENGINE-19`, are archived. The 18/19 creators are deliberately deferred until the 16/17 reviewer production write sets are closed.
- `EXR-UPCASE-20` and `EXR-BITMAP-21` architect packets are archived and launched as command-free follow-on work while runtime checker lanes proceed.

## Wave Record

- Scheduling or planning changes made in this wave:
  - resumed from `clear-forge`
  - treated `stone-lantern` as rollback history and `amper-ledger` as hand-restored protocol history, not a rollback target
  - validated the live owner-first board shape against the reset architect proposal
  - chose `EXR-FS-CORE-16` as the first packet-preparation target
  - archived and launched paired Wave A architect packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - `EXR-FS-CORE-16` architect lane launched
  - `EXR-FS-CORE-16` architect artifact returned from `Lorentz`, reconciled with `EXR-INODE-CORE-17`, and accepted
  - `EXR-INODE-CORE-17` architect lane launched
  - `EXR-INODE-CORE-17` architect artifact returned from `Newton`, reconciled with `EXR-FS-CORE-16`, and accepted
  - `COMPONENT_INDEX.md` advanced `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` from `Planned` to `Architected`
  - archived and launched paired Wave A designer packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`
  - no implementation or verification pass was launched
- Protocol, template, or packet-shaping changes made in this wave:
  - none
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - none

## Open Risks And Assumptions

- The board is accepted as a scheduling baseline and has now passed one owner-first architect wave; it has not yet been pressure-tested by designer or creator work.
- `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` are tightly coupled around trait-carrier state. They can be architected together and may have designer preparation in parallel once the shared owner contract is stable. Implementation should still be sequenced or explicitly sliced so `ExfatFs` owns shared state before `ExfatInode` relies on it, unless the architect records a narrower temporary seam.
- The Wave A designer packets should carry forward only the prior slices needed to specify the accepted `ExfatFs` / `ExfatInode` core ownership and VFS carrier obligations. They should not reopen unrelated Linux semantic details.
- The `EXR-FS-CORE-16` designer packet must preserve the accepted `root_inode()` temporary seam and exit plan, because the board's `EXR-FS-CORE-16` dependency list currently names only `EXR-BOOT-01` while the trait carrier surface needs an inode object.
- Runtime verification environment, Docker container state, and KVM status still need revalidation before any checker command lane is launched.

## Recommended Next Actions

1. Prepare designer packets for `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`, carrying forward the accepted architect seams and the shared `mod.rs` collision note.
2. Consider whether to run designer work in parallel. It is probably safe as command-free artifact work, but creator packet preparation must serialize the `mod.rs` declaration if both designs need new production files.
3. Keep `EXR-INODE-CACHE-18` out of the first wave unless the `EXR-FS-CORE-16`/`EXR-INODE-CORE-17` architect results show a narrow, owner-justified cache-key slice with no staging drift.

## Resume Checklist

- Read `README.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Read the active work-slice matrix in that handoff before dispatching or reshaping any lanes.
- Verify the environment summary above still matches reality.
- Confirm this handoff already reflects the material implementation and protocol changes from this wave before committing or handing off.
