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

- [x] **Phase 2: Architect Handoff** (`macro_00_global_topology.md`)
  - **Status**: Accepted after bounded repair and second structural audit
  - **Dispatch**: Initial `subagent-tasks/architect-global-topology/architect_global_topology_dispatch.md`; repair `subagent-tasks/architect-global-topology/architect_global_topology_repair_dispatch.md`
  - **Accepted Artifacts**: `components/architect-global-topology/macro_00_global_topology.md` and 13 meso architecture maps
  - **Acceptance Evidence**: 81 formal micro-feature IDs with unique primary ownership; complete meso sections; explicit same-level `Arc::as_ptr()` ordering; `DIR -> CUL -> INODE -> WL -> UPPER` topology; overlay-owned BIO-capable `Mutex` domains; no legacy implementation or ktest artifact dependency

- [x] **Phase 3: Designer Contracts** (per meso-component)
  - **Status**: Accepted after all 13 meso contract pairs passed structural audit
  - **Prerequisite**: Phase 2
  - **Accepted Artifacts**: 13 `_designer_spec.md` files and 13 `_designer_validation.md` files under `.agents/components/<component>/`
  - **Acceptance Evidence**: All 81 Architect-owned Micro IDs are covered exactly once by the matching Designer validation contracts; every pair has the required sections; validation is xfstests-only and many-to-many; no production code, ktest, or filesystem-local test surface was created

## 2. Meso-Component Pipeline Index

| Meso-Component | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :------------- | :--------------: | :------------------: | :---------------: | :---------------: | :-----------------: | :---------: | :------------- |
| `mount_options` | Accepted | Accepted | - | - | - | - | Specified |
| `mount_layers_lifecycle` | Accepted | Accepted | - | - | - | - | Specified |
| `identity_and_carriers` | Accepted | Accepted | - | - | - | - | Specified |
| `path_lookup_visibility` | Accepted | Accepted | - | - | - | - | Specified |
| `merged_readdir_cache` | Accepted | Accepted | - | - | - | - | Specified |
| `copy_up_lifecycle` | Accepted | Accepted | - | - | - | - | Specified |
| `file_io_page_cache` | Accepted | Accepted | - | - | - | - | Specified |
| `inode_attributes_security` | Accepted | Accepted | - | - | - | - | Specified |
| `directory_mutations_whiteouts` | Accepted | Accepted | - | - | - | - | Specified |
| `xattr_namespace_escaping` | Accepted | Accepted | - | - | - | - | Specified |
| `upper_exclusivity_durability` | Accepted | Accepted | - | - | - | - | Specified |
| `origin_index_export` | Accepted | Accepted | - | - | - | - | Specified |
| `metacopy_verity_data_layers` | Accepted | Accepted | - | - | - | - | Specified |

_Phase 3 Designer contracts are accepted. No implementation pass is active; the next schedulable action is main-agent pass slicing._

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
- The initial Architect artifacts were rejected for undefined same-level lock
  ordering, vague UPPER lock granularity, owner/boundary splits for root
  construction and mount-level identity policy, and a forbidden legacy
  `fs.rs` citation. The bounded repair resolved all four findings and the
  second audit passed; Phase 2 is accepted. No implementation work may start
  until meso-level Designer artifacts are accepted.
- Designer wording is intentionally xfstests-only: validation contracts map
  assigned micro-features to upstream xfstests, record externally observable
  runtime/integration evidence, and record missing upstream coverage. They do
  not request or imply internal unit tests, ktests, or filesystem-local test
  substitutes.
- The accepted Macro topology now includes a non-semantic Rust representation
  supplement: owner names are responsibility and traceability keys, not a
  one-to-one requirement for top-level Rust structs. Creators may merge,
  embed, or split private records using lifetime, lock, mutation-authority,
  and invariant analysis, while preserving the accepted owner/micro mapping,
  meso parent, publication authority, and lock topology.
- Designer dispatch initially hit the scheduler capacity limit. The main
  agent waited for actual completion signals, then continued in bounded waves;
  all 13 Designer pairs are now accepted. No completion was inferred from
  silence.
