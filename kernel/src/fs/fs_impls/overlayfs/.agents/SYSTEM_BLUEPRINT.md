<!-- SPDX-License-Identifier: MPL-2.0 -->

# System Blueprint & Dispatch Ledger

This file is the scheduler-owned live board for a filesystem implementation or refactor workspace.
Managers and subagents update artifacts elsewhere; only the main agent updates this board.

## 1. Macro Topology & Global Status

- [x] **Phase 0: Agent Workflow Bootstrap**
  - **Status**: Accepted
  - **Commit**: `b2d0df22a`
  - **Accepted Artifact**: `.agents/` directory layout, protocol, skills

- [x] **Phase 1: Priors Layer**
  - **Status**: Accepted
  - **Commit**: `f03ef716b` (priors) + `45362e19f` (ra-code-nav) + `ff1c00a2f` (xfstests mapping)
  - **Accepted Artifacts**: `priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`,
    `priors/FILESYSTEM_SPEC_SUMMARY.md`, `priors/FILESYSTEM_SPEC_INDEX.md`,
    `priors/MICRO_FEATURE_INVENTORY.md`, `.agents/skills/ra-code-nav/`

- [x] **Phase 2: Architect Handoff (reframed topology)** (`architect-reframed-topology`)
  - **Status**: Accepted after fresh structural audit; supersedes the deleted old topology
  - **Dispatch**: `subagent-tasks/architect-reframed-topology/architect_reframed_topology_dispatch.md`
  - **Accepted Artifacts**: `components/architect-reframed-topology/macro_00_global_topology.md` and 7 Meso architecture maps
  - **Acceptance Evidence**: 81 formal Micro IDs occur exactly once as primary owners; all maps contain the required architecture sections; the fresh `DIR -> CUL -> INODE -> WL -> UPPER` topology and Asterinas BIO/re-entry constraints are explicit; no implementation or test artifact was created
  - **Scope Classification**: 56 Micro IDs are `需要实现` (all P0/P1 plus
    `P2-01 xino`); 25 are `暂不实现` (`P2-02..P2-17` and `P3-01..P3-09`).
    Deferred IDs remain mapped future insertion points and are not
    implementation commitments in this wave.

- [ ] **Phase 3: Designer Contracts** (per new Meso-component)
  - **Status**: Not started; intentionally reset and no Designer artifact has been generated
  - **Prerequisite**: Accepted Phase 2 reframed topology
  - **Expected Artifacts**: None in this wave. Any future Designer work requires a new explicit dispatch under the 7 new Meso boundaries
  - **Boundary**: Do not reconstruct the deleted 13-Meso specs/validation contracts or create replacement Designer artifacts during this topology reset. The 56/25 classification is a design-scope decision only; it does not start Designer work.

## 2. Meso-Component Pipeline Index

| Meso-Component | Current Scope | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :------------- | :----------: | :-------------: | :------------------: | :---------------: | :---------------: | :-----------------: | :---------: | :------------- |
| `mount_resource_policy` | 8 / 7 | Accepted | Not started | - | - | - | - | Architected |
| `visibility_projection_identity` | 11 / 2 | Accepted | Not started | - | - | - | - | Architected |
| `merged_directory_index` | 4 / 1 | Accepted | Not started | - | - | - | - | Architected |
| `copyup_authority_file_views` | 17 / 6 | Accepted | Not started | - | - | - | - | Architected |
| `metadata_security_xattr_policy` | 4 / 4 | Accepted | Not started | - | - | - | - | Architected |
| `namespace_mutation_whiteout` | 11 / 1 | Accepted | Not started | - | - | - | - | Architected |
| `persistent_association_export` | 1 / 4 | Accepted | Not started | - | - | - | - | Architected |

`Current Scope` is shown as `需要实现 / 暂不实现`; the seven rows sum to
`56 / 25` across the complete 81-Micro inventory.

_The old 13-Meso topology and its Designer dispatch/specification artifacts are
discarded. No implementation, Creator, Checker, Reviewer, or pass-slicing work
is active; the next design action, if authorized, is a fresh Designer wave under
the seven accepted Meso boundaries._

## 3. Active Pass Tracking

Record only active or recently changed passes here. Durable slicing rationale belongs in `PASS_SLICING.md`.

- **`pass_00_old_ovfs_baseline_test`** — **Complete / Baseline Evidence Only**
  - **Kind**: Pre-design legacy baseline Checker pass; not an implementation pass
  - **Parent**: `legacy_overlayfs_baseline_validation` (temporary validation lane, not an Architect meso-component)
  - **Scope**: `overlay/001`–`overlay/078`, `overlay/100`, and `overlay/101`; classify every case as `PASS`, `FAIL`, `CORRUPT`, `NOTRUN`, or `HANG/TIMEOUT`
  - **Dispatch**: Follow-up agent `019f7f02-e0a0-7c90-9b4f-9083691caa6b` completed the attributable rerun; prior diagnostic agent `019f7ebb-09f6-7c22-b31a-1157328a91ea` is also closed
  - **Execution shape**: One fresh QEMU and freshly recreated TEST/SCRATCH image pair per case, a 300-second hang cutoff, and immediate append to the rerun status table
  - **Result**: 80 attributable rows: 9 `PASS`, 15 `FAIL`, 49 `NOTRUN`, and 7 `HANG/TIMEOUT`; no `CORRUPT`. The Architect design wave is now unblocked as a separate user-authorized scheduling decision.

## 4. Open Escalations / Notes

- Protocol decision recorded for the design wave: Macro/Meso/Micro remain the
  owner, lock-topology, semantic-boundary, traceability, and scheduling
  hierarchy; they are not xfstests granularity levels. xfstests is treated as
  a many-to-many external evidence mapping.
- The Designer validation contract is simplified to that external mapping plus
  runtime invariant and meso-integration observations. Creator-synced Checker
  passes retain exact code/micro scope for failure attribution, but do not
  require one test per micro-feature or any ktest-based validation. This
  refactor uses xfstests only.

- The legacy baseline is intentionally scheduled before the Architect wave, following the closed priors handoff. It does not advance Phase 2 or Phase 3 and cannot accept refactor implementation work.
- User-directed execution constraint satisfied: one QEMU and freshly recreated TEST/SCRATCH images were used per case; no shared batch was used.
- Diagnostic checkpoint: the initial 34 rows are provisional and invalid because image freshness was not proven and post-QEMU mount disappearance was misclassified as `CORRUPT`.
- Controlled rerun complete: classification used only each case's `qemu.log`; no `CORRUPT` was recorded. The one-hour controller ceiling was reconciled from the uniquely attributable `overlay/076` log, and the final 80-row matrix is preserved in `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv`.
- The previously accepted 13-Meso Architect/Designer wave is superseded by a
  user-directed topology reset. Its old component directories and old
  Designer dispatch packets were deleted; the retained old-OVFS baseline is
  independent evidence and is not an accepted refactor component.
- The fresh Architect output contains one Macro and seven Meso maps under
  `components/architect-reframed-topology/`. A mechanical audit found all 81
  formal Micro IDs exactly once, no duplicate or missing primary owner, no
  `P3-10`, and complete Macro/Meso architecture sections.
- No Designer spec/validation, Creator/Checker/Reviewer packet, production
  code, test, ktest, xfstests harness, build, or runtime artifact was created
  in the topology reset. Phase 3 remains intentionally unstarted.
- The global lock hierarchy accepted for this wave is
  `DIR -> CUL -> INODE -> WL -> UPPER`; `IU` remains an out-of-band mount-time
  upper/workdir claim protocol. BIO-capable paths use sleep-capable Mutex
  domains, and VFS dentry children guards are not parent locks.
- B/C-1 records two unresolved upper/workdir claim-carrier options: an
  inode-owned `Extension` runtime lease and a persistent xattr reservation.
  Neither is treated as an existing Asterinas capability or frozen API.
- Stage-D scope is now classified explicitly: implement all P0/P1 Micro IDs and
  `P2-01 xino` (56 total); defer `P2-02..P2-17` and `P3-01..P3-09` (25 total).
  The deferred set remains visible in the seven architecture maps for future
  Designer decisions, but no deferred Micro is part of the current
  implementation promise.
- This classification does not create Creator/Checker/Reviewer passes. Phase 3
  remains `Not started`, and any future Designer wave must use the seven Meso
  boundaries and preserve the same scope labels.
