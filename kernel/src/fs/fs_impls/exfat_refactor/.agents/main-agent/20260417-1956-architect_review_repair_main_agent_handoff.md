<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: Architect Review Repair for Inventory Clusters A-H, Cross-Prior Review, and Reading Guides

**Date / Time:** April 17, 2026, 22:23 CST
**Status:** Handed Over

## 1. Global State Pointer
*Always read `SYSTEM_BLUEPRINT.md` for the overarching project state. This section only notes immediate shifts made during this thread.*
- **Current Active Wave / Pass:** Prior Layer - Cross-Cutting Reading Guide Review Repair
- **Blueprint Updates Made:** No. This session repaired prior-layer inventory rows only and did not advance `SYSTEM_BLUEPRINT.md` or any Architect/Designer/Creator pass.

## 2. Pass Slicing Decisions
*Record the non-default pass boundaries chosen by the main agent. This is mandatory whenever a meso-component is split into multiple Creator/Checker passes.*
- `meso_01_mount_volume_state` is intentionally sliced as one complete Creator pass rather than nine separate micro-passes, because the accepted Designer contract exposes one exact meso interface and the user confirmed there is no need to cut this one micro-by-micro.
- `pass_01_mount_volume_state` covers exactly these micro-features: `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `VolumeDirty marks in-flight versus quiesced global state`; `VolumeFlags also carries media-failure and clear-before-modify state`; `Up-case Table is the durable case-folding truth source`; `Mount option defaults and remount mutability boundary`; `Superblock counters and statfs reflect cached cluster accounting`; `Asterinas mount lifecycle must eagerly expose root inode and global sync state`; `Mount-time accounting may fall back to recount under corruption-recovery conditions`.
- Creator and Checker for this pass must stay serialized one role at a time because they are likely to touch the same Rust files. Do not start the matching Checker until the Creator report and write-set have been main-agent reviewed.

## 3. Thread Activity Log (The Active Wave)
*What did this specific main-agent session actually do? Keep it concise. Focus on scheduling and dispatching.*
- **Dispatches Sent:**
  - User-requested Architect review subagent -> review-only read of Cluster A inventory rows; no official Architect packet or artifact was created.
  - User-requested Architect review subagent -> review-only read of Cluster B inventory rows immediately after the main-agent wrote them, matching the round-trip review workflow for inventory work.
  - User-requested Architect review subagent -> review-only read of Cluster C inventory rows immediately after the main-agent wrote them, matching the standing round-trip review workflow for inventory work.
  - Reused the existing Architect review thread -> review-only read of Cluster D inventory rows immediately after the main-agent wrote them, preserving prior context instead of restarting a fresh Architect subagent.
  - Reused the existing Architect review thread -> review-only read of Cluster E inventory rows immediately after the main-agent wrote them, preserving prior context instead of restarting a fresh Architect subagent.
  - Reused the existing Architect review thread -> review-only read of Cluster F inventory rows immediately after the main-agent wrote them, preserving prior context instead of restarting a fresh Architect subagent.
  - Reused the existing Architect review thread -> review-only read of Cluster G inventory rows immediately after the main-agent wrote them, preserving prior context instead of restarting a fresh Architect subagent.
  - Reused the existing Architect review thread -> review-only read of Cluster H inventory rows after the main-agent discovered the cluster had been written but not yet reviewed because the previous round appears to have been interrupted.
  - Reused the existing Architect review thread -> review-only pass over the full current inventory against the Microsoft index, Linux implementation summary, and Asterinas integration priors, focusing on rows whose wording had become too Linux-carrier-shaped for Architect consumption.
  - Reused the existing Architect review thread -> review-only read of the new cross-cutting reading-guide chapter that indexes existing rows for concurrency/lock-boundary and carrier-mismatch themes without adding new feature records.
  - Main-agent local documentation update -> recorded non-normative Macro/Meso generation rules in this live handoff and expanded the `macro_00_global_topology` template so the first official Architect wave can derive owner projection and candidate meso boundaries explicitly.
  - Official Architect dispatch -> archived `macro_00_global_topology` packet at `.agents/subagent-tasks/macro_00_global_topology/macro_00_global_topology_architect_dispatch.md` and launched the Architect lane to produce `.agents/components/macro_00_global_topology/macro_00_global_topology.md`.
- **Acceptance Outcomes:**
  - `Prior Layer - Architect review of Cluster A inventory rows` -> Accepted as review input. Reported three findings: missing backup-boot semantics / terminology drift, incomplete `VolumeFlags` coverage, and an over-strong remount statement around `zero_size_dir`.
  - `Prior Layer - Main Agent - Cluster A review repair` -> Accepted. Repaired `INV-PHY-001`, narrowed `INV-VFS-001`, and added `INV-PHY-004` to cover the missing `VolumeFlags` state bits.
  - `Prior Layer - Main Agent - Cluster B lookup / name encoding / dentry coherence inventory rows` -> Accepted. Added `INV-PHY-005` plus `INV-VFS-005` through `INV-VFS-010` covering the durable Up-case Table anchor, `iocharset`-selected lookup pipeline, `i_pos` identity rules, negative-dentry revalidation, case-folded hash/compare behavior, trailing-dot refusal boundaries, and alias-reuse behavior.
  - `Prior Layer - Architect review of Cluster B inventory rows` -> Accepted as review input. Reported two findings: missing explicit `NameHash` prefilter/full-compare wording and an under-specified alias-reuse state split.
  - `Prior Layer - Main Agent - Cluster B review repair` -> Accepted. Tightened `INV-VFS-008` and `INV-VFS-010` to reflect the Architect review findings without widening the scope beyond the cited priors.
  - `Prior Layer - Main Agent - Cluster C allocation / size mutation / write ordering inventory rows` -> Accepted. Added `INV-PHY-006` plus `INV-VFS-011` through `INV-VFS-016` covering persisted stream state, `fallocate` refusal, non-contiguous growth / `NoFatChain` flip, `valid_size` zero-fill, append write ordering, truncate/shrink semantics, and Asterinas inode-surface pressure.
  - `Prior Layer - Architect review of Cluster C inventory rows` -> Accepted as review input. Reported two findings: missing explicit truncate/shrink coverage and incomplete spec anchoring for the write-ordering row.
  - `Prior Layer - Main Agent - Cluster C review repair` -> Accepted. Added the missing truncate/shrink row and attached the Microsoft `8.1 Recommended Write Ordering` anchor to `INV-VFS-014`.
  - `Prior Layer - Main Agent - Cluster D directory lifecycle / tree mutability inventory rows` -> Accepted. Added `INV-PHY-007` plus `INV-VFS-017` through `INV-VFS-023` covering entry-set continuity, create/mkdir slot acquisition, unlink/rmdir ordering, cross-directory rename sequencing, directory emptiness gates, Linux rename-flag boundaries, `zero_size_dir`, and Asterinas path-tree integration realities.
  - `Prior Layer - Architect review of Cluster D inventory rows` -> Accepted as review input. Reported three findings: missing emptiness-gate coverage, missing Asterinas-local rename-surface clarification, and a weak spec anchor on the deletion-ordering row.
  - `Prior Layer - Main Agent - Cluster D review repair` -> Accepted. Added the emptiness-gate row, split Linux rename-flag semantics from the Asterinas-local rename surface, and strengthened the deletion-ordering row with the Microsoft write-ordering anchor.
  - `Prior Layer - Main Agent - Cluster E page-cache / block mapping / runtime I/O inventory rows` -> Accepted. Added `INV-VFS-024` through `INV-VFS-027` plus `INV-BIO-001` and `INV-BIO-002`, covering block mapping, sync semantics, direct-I/O alignment, generic runtime-I/O delegation, page-cache backend mapping, and BIO wait boundaries.
  - `Prior Layer - Architect review of Cluster E inventory rows` -> Accepted as review input. Reported two findings: missing truncate-versus-block-mapping synchronization coverage and missing Asterinas spinlock-vs-BIO prohibition coverage.
  - `Prior Layer - Main Agent - Cluster E review repair` -> Accepted. Added `INV-VFS-028` for the truncate/block-mapping synchronization boundary and `INV-BIO-003` for the `SpinLock`/BIO incompatibility rule.
  - `Prior Layer - Main Agent - Cluster F permissions / ownership / timestamp / timezone inventory rows` -> Accepted. Added `INV-PHY-008` plus `INV-VFS-029` through `INV-VFS-032`, covering DOS-style attribute and timestamp persistence, mount-derived ownership and mode, `ATTR_RO` mapping, timestamp/timezone translation, and Asterinas metadata-surface pressure.
  - `Prior Layer - Architect review of Cluster F inventory rows` -> Accepted as review input. Reported two findings: missing `ctime`-surface mismatch coverage and missing `allow_utime` timestamp-policy coverage.
  - `Prior Layer - Main Agent - Cluster F review repair` -> Accepted. Added `INV-VFS-033` for timestamp-mutation policy and tightened `INV-VFS-032` to call out the `ctime`-surface mismatch explicitly.
  - `Prior Layer - Main Agent - Cluster G administrative ABI / unsupported / refusal boundary inventory rows` -> Accepted. Added `INV-PHY-009` plus `INV-VFS-034` through `INV-VFS-039`, covering persistent administrative metadata, DOS-attribute ioctls, `FITRIM` versus online discard, forced shutdown, volume-label ABI, typed refusal boundaries, and the current Asterinas-side carrier gap for Linux-shaped management ABIs.
  - `Prior Layer - Architect review of Cluster G inventory rows` -> Accepted as review input. Reported two findings: missing explicit Asterinas-side carrier-gap coverage and an over-broad refusal-taxonomy row.
  - `Prior Layer - Main Agent - Cluster G review repair` -> Accepted. Added the Asterinas administrative-carrier gap row and tightened the refusal-taxonomy row so unsupported, invalid, and read-only refusals stay distinct.
  - `Prior Layer - Main Agent - Cluster H consistency / recovery / anomaly surface inventory rows` -> Accepted. Added `INV-VFS-040` through `INV-VFS-043`, covering append crash windows, cross-directory rename crash windows, corruption-recovery recount fallback, and anomaly-state handling for dirty/media-failure/forced-shutdown paths.
  - `Prior Layer - Architect review of Cluster H inventory rows` -> Accepted as review input. Reported two findings: missing `ClearToZero` coverage in the anomaly-state row and missing `8.2 Implications of Unrecognized Directory Entries` coverage for typed anomaly boundaries around unknown entries.
  - `Prior Layer - Main Agent - Cluster H review repair` -> Accepted. Expanded `INV-VFS-043` to include `ClearToZero` and added `INV-VFS-044` for the spec-defined invalidity/no-modify boundaries around unrecognized directory entries.
  - `Prior Layer - Architect cross-prior review of current inventory wording` -> Accepted as review input. Reported five findings: Linux dcache internals were over-recorded as if target-neutral (`INV-VFS-007`, `INV-VFS-010`), fallocate refusal was over-bound to the Linux carrier (`INV-VFS-011`), stable inode identity was phrased as Linux `i_pos` machinery (`INV-VFS-006`), the block-mapping anti-race row over-prescribed Linux lock shape (`INV-VFS-028`), and the ordinary runtime-I/O row was still too Linux file-operations/helper-shaped (`INV-VFS-027`).
  - `Prior Layer - Main Agent - Cross-prior wording repair` -> Accepted. Rewrote `INV-VFS-006`, `INV-VFS-007`, `INV-VFS-010`, `INV-VFS-011`, `INV-VFS-027`, and `INV-VFS-028` so their guarantees stay target-neutral while Linux/Asterinas carrier differences are now confined to source anchors or notes.
  - `Prior Layer - Main Agent - Cross-cutting reading-guide chapter` -> Accepted. Added `## 6. Cross-Cutting Reading Guide` with two non-feature navigation sections: one for concurrency / lock-boundary / sleep-boundary rows and one for carrier-mismatch / target-surface-pressure rows.
  - `Prior Layer - Architect review of cross-cutting reading guide` -> Accepted as review input. Reported two findings: one grouping over-read deletion/rename ordering rows as if they already established concurrent namespace-change semantics, and another grouping misleadingly mixed `fsync` with the mutable-stream anti-race set.
  - `Prior Layer - Main Agent - Cross-cutting reading-guide repair` -> Accepted. Narrowed the directory/tree grouping back to ordering/overwrite gates and split `fsync` into its own navigation line while moving the mutable-stream grouping to `INV-VFS-024`, `INV-VFS-027`, and `INV-VFS-028`.
  - `Prior Layer - Main Agent - targeted tail-wording cleanup` -> Accepted. Tightened `INV-VFS-021`, `INV-VFS-025`, `INV-VFS-027`, and `INV-VFS-036` so the rows now keep Linux control-flow and runtime side effects subordinate to the stable micro-feature instead of presenting them as target-neutral invariants.
  - `Prior Layer - Main Agent - final tail-wording cleanup` -> Accepted. Tightened `INV-VFS-008` and `INV-VFS-026` so the remaining lookup and direct-I/O rows no longer depend on Linux dentry/entry-point naming or over-specific alignment-gate phrasing in their primary invariants.
  - `Architect kickoff preparation - Macro/Meso working rules handoff section + template expansion` -> Accepted. Added a non-normative section in this live handoff that defines the current Macro/Meso generation order and meso boundary heuristic, and updated the `macro_00_global_topology` template to require On-disk Structure Owners, Runtime Owners, an ownership projection matrix, and a candidate meso index before the lock hierarchy.
  - `Architect kickoff dispatch - macro_00_global_topology` -> Dispatched. `SYSTEM_BLUEPRINT.md` Phase 1 status moved from Pending to Dispatched to Architect; no Architect artifact has been accepted yet.
  - `Architect execution - macro_00_global_topology` -> Artifact written, review pending. Produced `.agents/components/macro_00_global_topology/macro_00_global_topology.md` with On-disk Structure Owners, Runtime Owners, the owner-projection matrix, candidate meso boundaries, and a revised static lock hierarchy after one self-repair iteration corrected the allocator-lock position relative to inode/stream locks.
  - `Architect repair request 01 - macro_00_global_topology` -> Dispatched. Main-agent requested a focused second Architect pass to normalize every primary-owner field to one owner, tighten the temporary-seam story around `ExfatAdminAdapter`, and make the `filesystem_sync_and_volume_state` vs `file_sync_and_persistence` split explicit enough for acceptance.
  - `Architect repair 01 result - macro_00_global_topology` -> Accepted. The repaired artifact removed `ExfatAdminAdapter` as a primary Runtime Owner, normalized primary-owner fields to single owners, split file and directory metadata projection, and clarified the file-sync versus filesystem-sync meso boundary.
  - `Blueprint update - Phase 1 accepted` -> Accepted. `SYSTEM_BLUEPRINT.md` now marks Phase 1 as accepted/frozen and seeds eleven planned meso components from the accepted macro topology.
  - `Architect dispatch - meso_01_mount_volume_state` -> Dispatched. Archived `.agents/subagent-tasks/meso_01_mount_volume_state/meso_01_mount_volume_state_architect_dispatch.md` and marked `meso_01_mount_volume_state` Architect Map as in progress in `SYSTEM_BLUEPRINT.md`.
  - `Architect result - meso_01_mount_volume_state` -> Accepted. The artifact covers mount/bootstrap/root/superblock micro-features, preserves the macro lock hierarchy, and explicitly leaves runtime discard/accounting, steady-state lookup, and filesystem-wide sync transitions to later meso components.
  - `Parallel Architect dispatch wave - meso_02/03/08` -> Dispatched. Archived three Phase 2 Architect packets for `meso_02_free_space_accounting_and_discard`, `meso_03_directory_lookup_and_identity`, and `meso_08_filesystem_sync_and_volume_state`, then marked their Architect Map lanes in progress in `SYSTEM_BLUEPRINT.md`.
  - `Architect result - meso_02_free_space_accounting_and_discard` -> Accepted. The artifact maps the allocator / `statfs` / discard / `FITRIM` / recount rows onto one `ExfatFs` meso, keeps mount seeding in `meso_01`, and leaves forced-shutdown state transitions as imported overlays rather than stealing volume-state ownership.
  - `Architect result - meso_03_directory_lookup_and_identity` -> Accepted. The artifact maps lookup / readdir / identity / alias / negative-cache / unrecognized-entry rows onto one `ExfatInode(dir)` read-side namespace meso, consumes the Up-case Table and root identity from `meso_01`, and keeps create/unlink/rename mutation semantics out of scope.
  - `Architect result - meso_08_filesystem_sync_and_volume_state` -> Repair requested. The artifact correctly kept file-scoped `fsync`, forced-shutdown carrier, and discard paths out of scope, but main-agent review found it must explicitly account for `INV-VFS-004` filesystem-wide sync semantics and `INV-PHY-010` static VolumeDirty write-ordering brackets before acceptance.
  - `Blueprint update - meso_02/03 accepted, meso_08 repair requested` -> Accepted. `SYSTEM_BLUEPRINT.md` now marks the Architect Map lane as accepted for `meso_02_free_space_accounting_and_discard` and `meso_03_directory_lookup_and_identity`, while `meso_08_filesystem_sync_and_volume_state` is back in Architect repair.
  - `Architect repair 01 result - meso_08_filesystem_sync_and_volume_state` -> Accepted. The repaired artifact now explicitly accounts for `INV-VFS-004` by assigning continuing `FileSystem::sync()` semantics to `meso_08` while leaving root publication to `meso_01`, and accounts for `INV-PHY-010` as a global `VolumeDirty` persistence bracket without stealing local mutation ordering from `meso_04` or `meso_06`.
  - `Blueprint update - meso_08 accepted` -> Accepted. `SYSTEM_BLUEPRINT.md` now marks the Architect Map lane as accepted for `meso_08_filesystem_sync_and_volume_state`.
  - `Parallel Designer/Architect dispatch wave - meso_01 / meso_04 / meso_06` -> Dispatched. Archived one Designer packet for `meso_01_mount_volume_state` and two Phase 2 Architect packets for `meso_04_directory_entry_mutation` and `meso_06_file_content_mutation`, then marked those lanes in progress in `SYSTEM_BLUEPRINT.md`.
  - `Designer result - meso_01_mount_volume_state` -> Repair requested. The artifacts preserve the right ownership boundaries and provide useful ktest obligations, but the Designer spec violates the exact-interface requirement by allowing the Creator to choose concrete type names for the meso-level interface.
  - `Architect result - meso_04_directory_entry_mutation` -> Accepted. The artifact maps namespace mutation, entry-set continuity, create/mkdir slot acquisition, unlink/rmdir ordering, rename sequencing, emptiness gates, newborn directory shape, Asterinas refusal surfaces, rename crash windows, and mutation-side unrecognized-entry boundaries onto one `ExfatInode(dir)` mutation meso while keeping lookup, allocator, and volume-state ownership imported from siblings.
  - `Architect result - meso_06_file_content_mutation` -> Accepted. The artifact groups Stream Extension mutation, `NoFatChain` flip, zero-fill, append publication ordering, shrink semantics, direct-I/O gating, mutation-side truncate anti-race, and explicit fallocate refusal under one `ExfatInode(file)` mutation meso while importing allocator and volume-state overlays from accepted siblings.
  - `Blueprint update - meso_01 Designer repair requested / meso_04 and meso_06 Architect accepted` -> Accepted. `SYSTEM_BLUEPRINT.md` now marks `meso_01_mount_volume_state` Designer Contract as under repair, while `meso_04_directory_entry_mutation` and `meso_06_file_content_mutation` Architect Maps are accepted.
  - `Designer repair 01 result - meso_01_mount_volume_state` -> Accepted. The repaired Designer spec now provides one exact crate-visible signature with fixed contract types and exact `MountVolumeStateError::*` variants, and the ktest artifact now asserts against those same exact variants.
  - `Blueprint update - meso_01 Designer accepted` -> Accepted. `SYSTEM_BLUEPRINT.md` now marks `meso_01_mount_volume_state` Designer Contract as accepted and ready for later pass slicing.
  - `Creator slicing decision - meso_01_mount_volume_state` -> Accepted. User confirmed `meso_01` may be implemented as one full Creator pass; archived `.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_creator_dispatch.md` and updated `SYSTEM_BLUEPRINT.md` to show `pass_01_mount_volume_state` as dispatched.
  - `Creator result - pass_01_mount_volume_state` -> Rejected for retry. The Creator returned a structurally valid blocker report, but main-agent review judged the blocker to be caused by packet ambiguity plus an over-strict ownership read: `meso_01_mount_volume_state` already owns root publication and may create the initial refactor substrate required for its accepted contract.
  - `Creator repair 01 dispatch - pass_01_mount_volume_state` -> Withdrawn. Main-agent later revoked this packet after user review because it incorrectly pointed the Creator at legacy `kernel/src/fs/fs_impls/exfat/` implementation files, violating the refactor intent and the Creator information funnel.
  - `Creator repair 02 dispatch - pass_01_mount_volume_state` -> Dispatched. Archived `.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_creator_repair_02_dispatch.md`, removed all legacy `exfat` references from Creator inputs, kept only stable Asterinas VFS interfaces plus the accepted Designer contract, and preserved the same pass scope and output report path.
  - `Creator repair 02 result - pass_01_mount_volume_state` -> Accepted. The Creator produced refactor-owned mount/runtime code under `kernel/src/fs/fs_impls/exfat_refactor/` plus a complete Creator report, stayed within the allowed write-set, did not consult legacy `exfat` implementation code, and left registry takeover disabled so protocol rule 7 still holds.
  - `Checker / Creator repair loop - pass_01_mount_volume_state` -> Accepted after multiple bounded iterations. Checker first hit environment/tooling blockers, then compile-path fixes, then baseline fixture validation failures. Main-agent only applied shallow compile/tooling repairs directly, routed production/diagnostic failures back through Creator/Checker packets, and preserved the no-legacy-`exfat` Creator funnel.
  - `Checker final result - pass_01_mount_volume_state` -> Accepted. Full `make kernel` passed in `codex-asterinas-dev`; all ten exact-name `mount_volume_state_*` ktests passed under checker lock; qemu serial receipts showed no panic, deadlock, RCU stall, or cyclic-lock evidence.
  - `Reviewer result - pass_01_mount_volume_state` -> Accepted. Reviewer made non-functional seam-comment and wrapping edits only, approved helper legality, and explicitly recorded that no additional Checker pass is required after review.
- **Escalations / Deadlocks:**
  - None.

## 4. Explicit Agent-Level Decisions
*Record non-automated choices made by the main agent during this session. (e.g., clearing stale locks, skipping final reviewer, reopening a Creator Pass after an integration failure).*
- Treated the Architect review as authoritative for inventory wording corrections, but still kept it outside the official Architect artifact pipeline.
- Upgraded `INV-PHY-001` from “main boot sector load” language to “main/back-up boot-region validation” language so the inventory reflects the Microsoft recovery boundary without implying any implementation-specific fallback path.
- Split the missing `VolumeFlags` facts into a new row instead of overloading the dirty-bit row, preserving factual granularity for future Architect use.
- Narrowed the remount-boundary record to only the option set explicitly supported by the current Linux prior and downgraded `zero_size_dir` to a mount-time semantic note rather than a remount claim.
- Chose to represent the second inventory round as one physical naming-state anchor (`Up-case Table`) plus six VFS-facing lookup / dentry facts, rather than forcing all name semantics into a single oversized row.
- Adopted a standing workflow for this tenure: every inventory-writing round must go through `main-agent write -> Architect review -> main-agent repair` before the round is considered stable.
- Folded the Cluster B repair directly into the same live handoff file instead of creating another handoff note, matching the single-live-file rule added to the protocol.
- Chose to represent the third inventory round as one persisted stream-state row plus six VFS/integration rows so that physical stream invariants stay separate from Linux/Asterinas behavior boundaries.
- Accepted the Architect guidance that Cluster C was missing shrink semantics and repaired that gap before treating the round as stable.
- Left the Asterinas inode-surface row in place as an interface-pressure fact, but kept its ownership note explicitly non-prescriptive so later Architect work does not over-read it as a topology decision.
- Reused the same Architect review thread for Cluster D instead of tearing it down and spawning a new one, so the review lane kept its accumulated prior context.
- Chose to split Cluster D's rename facts into three layers after review: namespace sequencing, directory-emptiness refusal, and the Linux-versus-Asterinas ABI surface difference around rename flags.
- Strengthened the deletion-ordering row with a write-ordering spec anchor rather than over-claiming that the `EntryType` definition alone justified the full sequencing rule.
- Kept reusing the same Architect review thread for Cluster E as well, so the reviewer did not have to re-ingest the prior-layer context from scratch.
- Split Cluster E across VFS and BIO layers, then accepted the Architect feedback that the first cut still lacked one synchronization fact and one hard substrate prohibition.
- Chose to record the truncate-versus-block-mapping boundary as a factual synchronization requirement without importing Linux's exact lock implementation into topology language.
- Kept reusing the same Architect review thread for Cluster F too, continuing the single-review-lane approach for consecutive inventory rounds.
- Chose to split Cluster F into one on-disk metadata row plus four VFS-facing rows, then accepted the Architect feedback that metadata policy still needed explicit `allow_utime` and `ctime`-mismatch coverage.
- Recorded the `ctime` gap as a target-VFS-versus-on-disk capability mismatch instead of pretending exFAT already has a native one-to-one field for it.
- Kept reusing the same Architect review thread for Cluster G as well, so the administrative-ABI review still benefited from the accumulated prior context.
- Accepted the Architect feedback that Linux-side administrative semantics and Asterinas-side carrier availability needed to be separated explicitly instead of being left implicit.
- Narrowed the refusal-boundary row so it now describes typed errno taxonomy rather than over-grouping unsupported, invalid, and read-only cases under one label.
- Treated the interrupted Cluster H round as a continuity problem rather than a reason to fork a new handoff or a new Architect thread: the main-agent reused the same review lane, confirmed the cluster state locally, and then completed the missing review/repair loop.
- Accepted the Architect feedback that Cluster H was too Linux-runtime-centric in its first cut and repaired it by adding the missing spec-side anomaly surfaces instead of only polishing wording.
- Kept the new unrecognized-directory-entry row factual and non-topological by recording only the typed invalidity, no-modify, and limited-directory-operation boundaries from Section 8.2.
- Used the first cross-prior whole-inventory review to tighten the inventory's language model: the durable micro-feature now lives in the `Required Invariant / Guarantee` field, while Linux-specific carrier realizations and Asterinas-surface differences are pushed down into notes/anchors unless the mismatch itself is the feature.
- Chose not to reopen the earlier Cluster A review findings during this pass because those rows were already repaired in the file; this round only touched the newly identified Linux-carrier overreach issues.
- Treated the user's proposed two tail items as a reading/navigation problem rather than as missing-feature clusters: the inventory now has a dedicated chapter for those cross-cutting themes, but it still does not pretend they are new source-backed rows.
- Accepted Architect's warning that even navigation chapters can accidentally over-synthesize, then narrowed the guide labels so they remain faithful to the underlying rows.
- In the targeted tail cleanup, recast `INV-VFS-021` as a Linux-prior ABI boundary without preserving the Linux VFS precheck narrative inside the invariant itself.
- Reframed `INV-VFS-025` around the persistence boundary (`file writeback` + `block-device sync` + `device-cache flush`) instead of helper-stack narration.
- Rewrote `INV-VFS-027` so it records cached-I/O reuse plus exFAT boundary guards as the fact, not a recommendation against a filesystem-private data plane.
- Narrowed `INV-VFS-036` so forced shutdown remains the primary fact and later discard suppression is explicitly scoped to the current Linux prior's runtime behavior.
- Recast `INV-VFS-008` around name-match prefilter/comparison semantics so Linux `dentry` wording no longer appears as the target-neutral carrier.
- Reframed `INV-VFS-026` around filesystem/device alignment requirements and direct-I/O intent, removing Linux-specific entry-point names from the invariant.
- Closed this live note as a handoff boundary after the inventory-cleaning wave, with the prior layer considered sufficiently polished for Phase 1 Architect kickoff.
- Chose to record the new Macro/Meso generation heuristic in this live handoff instead of hardening it directly into `PROTOCOL.md` or adding another workspace-level note, because this is still a planning aid for the first official Architect wave rather than a scheduler rule proven by artifact review.
- Expanded the `macro_00_global_topology` template so the first Architect artifact must show the bridge from durable truths to runtime authorities to candidate meso boundaries before it freezes the lock hierarchy.
- Authorized the first official Architect packet for `macro_00_global_topology` to read the stabilized micro-feature inventory, Microsoft exFAT priors, Linux exFAT prior, Asterinas integration prior, and this handoff Section 5; the Architect is not authorized to edit `SYSTEM_BLUEPRINT.md` or create Phase 2 meso artifacts in this pass.
- Rejected the first `macro_00_global_topology` artifact for one focused repair round because the current artifact still leaves a composite primary-owner label in the metadata path, leaves the temporary administrative seam too open-ended for Phase 1 acceptance, and needs a crisper justification if filesystem-wide sync remains a distinct meso boundary.
- Accepted the repaired `macro_00_global_topology` as the frozen Phase 1 backbone because it now has singular Runtime Owners, explicit durable-owner projection, reproducible candidate meso boundaries, and a static lock hierarchy suitable for Phase 2 mapping.
- Assigned stable meso IDs `meso_01` through `meso_11` in `SYSTEM_BLUEPRINT.md` directly from the accepted candidate meso index; these IDs are scheduler-owned and should be used for Phase 2 Architect dispatches.
- Chose `meso_01_mount_volume_state` as the first Phase 2 Architect map because mount/bootstrap state is the root of later `ExfatFs` runtime authority, validated geometry, global flags, root-object creation, and mount/remount policy.
- Accepted `meso_01_mount_volume_state` without a repair round because it structurally matches the meso architecture template, names nine traceability rows, defines inlet/topology boundaries under the frozen macro hierarchy, and records downstream structural interactions without prescribing Designer choreography.
- Chose a parallel Architect wave for `meso_02`, `meso_03`, and `meso_08` because their write-sets are disjoint, they are command-free, and together they clarify the allocator, lookup, and filesystem-wide sync boundaries that `meso_01` Designer will need as neighboring contracts.
- Accepted `meso_02_free_space_accounting_and_discard` without a repair round because the artifact cleanly groups the allocator-accounting rows, records the trim/discard distinction, and preserves forced-shutdown as an imported sibling-state effect instead of collapsing volume-state ownership into the allocator meso.
- Accepted `meso_03_directory_lookup_and_identity` without a repair round because the artifact stays read-side, keeps the lookup codec / alias / identity rules together under `ExfatInode(dir)`, and leaves namespace mutation semantics to `meso_04`.
- Rejected `meso_08_filesystem_sync_and_volume_state` for a focused repair round because the first artifact did not explicitly account for `INV-VFS-004`'s filesystem-wide sync requirement or `INV-PHY-010`'s static VolumeDirty write-ordering bracket, even though its owner and sibling-boundary choices were otherwise sound.
- Accepted the repaired `meso_08_filesystem_sync_and_volume_state` because it now maps whole-filesystem `sync`, global dirty/quiesce state, and the static `VolumeDirty` bracket without reabsorbing `meso_01` mount publication, `meso_07` file-scoped `fsync`, or `meso_04` / `meso_06` local mutation ordering.
- Chose to start `meso_01_mount_volume_state` Designer now that its accepted neighboring Architect boundaries (`meso_02`, `meso_03`, `meso_08`) exist, while continuing command-free Architect work on mutation-heavy `meso_04` and `meso_06` in parallel because the write-sets are disjoint and those maps will de-risk the later Designer wave.
- Rejected `meso_01_mount_volume_state` Designer for a focused repair round because the spec's meso-level interface is not exact enough: it names `MountVolumeStateTarget`, `MountVolumeStateOperation`, and `MountVolumeStateOutcome`, then says the Creator may choose concrete type names, which leaves architecture guesswork in violation of the Designer protocol.
- Accepted the repaired `meso_01_mount_volume_state` Designer because the meso-level interface is now exact, the meso-local contract types and error variants are fixed in the spec, and the Checker obligations use those same exact variants instead of leaving type-shape discretion to the Creator.
- Accepted `meso_04_directory_entry_mutation` without a repair round because the artifact stays write-side, keeps allocator and volume-state ownership imported, preserves lookup/identity as a sibling boundary, and cleanly groups create/mkdir/unlink/rmdir/rename mutation semantics under one directory mutation contract.
- Accepted `meso_06_file_content_mutation` without a repair round because the artifact keeps allocator and volume-state ownership imported, places the truncate/mapping anti-race on the mutation side without revising macro topology, and cleanly binds fallocate refusal, direct-I/O gating, zero-fill, append publication, and shrink semantics into one file-mutation contract.
- Recorded the user's implementation-lane preference that `meso_01` can be cut as one full Creator pass instead of micro-pass-by-micro-pass, but Creator and Checker must remain serialized because they likely work in the same Rust files.
- Recorded the execution environment for future Checker packets: all compile/test execution must run in the Docker container `codex-asterinas-dev`; full compile uses `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`; filtered ktests use `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <ktest full name>'`; and all such execution must be protected by `.agents/tools/checker_lock.sh acquire` / `release`.
- Recorded the user instruction that all delegated subagents in this workflow must be launched with model `gpt-5.4`.
- Rejected the first `pass_01_mount_volume_state` Creator result as a retryable packet-boundary failure rather than a true topology blocker, because the accepted `meso_01` artifacts already place root publication inside this meso and the Creator packet had not supplied enough local Asterinas interface context to confidently build the initial refactor substrate.
- Initially chose a narrow Creator repair instead of escalating upward immediately, but later withdrew that repair packet after the user pointed out that Creator must not be steered toward the legacy `exfat` implementation.
- Recorded a stricter Creator-funnel rule in both `PROTOCOL.md` and `protocol/CREATOR.md`: Creator may read only the accepted Designer contract, code-quality prior, and stable pre-existing Asterinas kernel interfaces needed for typing/integration; Creator must not read or reuse legacy `kernel/src/fs/fs_impls/exfat/` implementation code as a design oracle.
- Closed the running `Feynman` Creator subagent on user request before it could continue from the withdrawn repair packet.
- Re-dispatched `pass_01_mount_volume_state` with a fresh packet that removes all legacy `exfat` references and restores the intended refactor-first implementation boundary.
- Accepted the repaired Creator result because it now builds a fresh refactor-owned substrate (`mod.rs`, `fs.rs`, `inode.rs`, `ondisk.rs`) under `kernel/src/fs/fs_impls/exfat_refactor/`, keeps root publication inside `meso_01` as required, and limits mount-visible directory behavior to the current pass boundary instead of silently absorbing later lookup/mutation ownership.
- Accepted the final green Checker result for `pass_01_mount_volume_state` after the bounded repair loop. Direct main-agent repairs during the loop were limited to shallow compile/import/tooling issues; non-shallow baseline validation failures were routed back to Creator or Checker.
- Accepted the Reviewer result for `pass_01_mount_volume_state` and skipped the post-review final Checker because Reviewer explicitly recorded non-functional edits only.
- Recorded a temporary workflow audit file at `.agents/tmp/20260419-meso_01_round_trace.md` so the user can inspect every subagent attempt and workflow incident from this round.
- Hardened the workflow protocol after the first implementation wave: Creator reports now require a complete introduced-entity census, Reviewer remains post-Checker but is split into a line-level quality gate plus a structural helper / owner-placement gate, broad structural cleanup is routed back to Creator instead of being performed inside Reviewer, and post-review final Checker is skippable only for explicitly line-level non-functional Reviewer edits.
- Added shared workflow tools for future agents: `.agents/tools/checker_run.sh` is now the preferred Docker-backed Checker runner for `make kernel` / exact-name ktests with per-test `qemu-serial.log` archiving, and `.agents/tools/ra_code_nav.py` is the preferred rust-analyzer LSP navigation helper for scoped Asterinas symbol lookup, file outlines, definitions, references, implementations, and hover/type information.

## 5. Non-Normative Macro/Meso Working Rules
*Planning aid only. This section records the current main-agent brainstorming for the first official Architect wave; it is not yet a scheduler protocol rule.*

### 5.1 Purpose

The inventory is now broad enough to start architectural generation, but the original `macro_00_global_topology` template was too thin to derive a clear Macro/Meso structure on its own. In particular, it mixed durable exFAT truth sources, runtime coordination authorities, and global lock hierarchy.

The working rule is to separate those steps so the Architect first names durable structures, then runtime ownership projection, then meso candidates, and only then freezes the lock topology.

### 5.2 Macro Generation Order

1. **On-disk Structure Owners**
   - List concrete durable exFAT structures or state machines that must exist as stable truths in the final system.
   - Examples include Boot region / `VolumeFlags`, Allocation Bitmap, FAT, Up-case Table, directory-entry set, Stream Extension, and volume label / volume GUID entries.
2. **Runtime Owners**
   - List runtime authorities that coordinate or project those durable truths into the running filesystem model.
   - Typical candidates are VFS trait carriers such as `ExfatFs`, `ExfatInode(file)`, and `ExfatInode(dir)`.
   - A temporary seam is acceptable only when it has an explicit exit plan.
3. **Ownership Projection**
   - Map each On-disk Structure Owner to its primary Runtime Owner.
   - Record secondary collaborators only when the distinction matters for later static boundaries.
   - This step prevents Owner Gaps and stops the Architect from inventing Macro-Owners that are neither durable truths nor stable runtime authorities.
4. **Candidate Meso Index**
   - Generate the first meso candidates from the ownership projection.
   - Each candidate should already look like a unit that can later receive one Architect traceability map and one Designer contract.
5. **Global Lock Topology**
   - Freeze the static macro-level lock hierarchy only after the owner and candidate-meso structure is visible.
   - The lock topology validates and constrains the structure; it should not be the only thing from which structure is inferred.

### 5.3 Meso Definition

A `meso` is the smallest responsibility boundary that is still large enough for all of the following to be true:

- the Architect can map an exhaustive micro-feature set into it,
- the Designer can write one coherent dynamic contract for it,
- and the main agent can later slice Creator / Checker passes inside it.

In working terms:

> A Meso-Component is one primary Runtime Owner handling one entry-surface family over one durable touch-set family under one static lock envelope and one consistency / failure domain.

This means a meso is smaller than a Macro-Owner, larger than any single micro-feature, and not defined by helper functions or private code layout.

### 5.4 Meso Generation Rule

Generate candidate meso components by grouping micro-features that are compatible on all five axes below:

1. **Primary Runtime Owner**
   - Who is the main runtime authority for this behavior?
2. **Entry-Surface Family**
   - Which external semantic family activates it?
   - Typical families include mount, lookup, read, content mutation, directory-entry mutation, metadata projection, sync, and administrative identity handling.
3. **Durable Touch-Set Family**
   - Which On-disk Structure Owners are materially touched or coordinated?
4. **Static Lock Envelope**
   - What inlet lock state is assumed, and what is the highest lock level this unit may legally acquire?
5. **Consistency / Failure Domain**
   - Do these micro-features need one shared sequencing, rollback, or anomaly contract?

If two groups of micro-features need different answers on any of these axes, they should normally become different meso components.

### 5.5 Practical Split / Merge Heuristics

Split into different meso components when:

- the primary Runtime Owner changes,
- the entry surfaces belong to different user-visible semantic families,
- the durable touch-set meaningfully changes,
- the static lock envelope changes,
- or the Designer would need different sequencing / failure contracts.

Keep in the same meso component when:

- the same Runtime Owner handles the same family of entry surfaces,
- the same durable state families are coordinated together,
- the same static lock assumptions hold,
- and the features must be specified together to avoid order or rollback ambiguity.

Do **not** split meso boundaries based on helper-function ideas, speculative implementation reuse, or private code organization preferences.

### 5.6 Cross-Cutting Overlay Rule

Not every inventory cluster should become its own meso component.

Cross-cutting recovery or anomaly facts should often remain overlay obligations attached to the meso components that actually trigger them. Examples include:

- `VolumeDirty` write-ordering brackets,
- media-failure or forced-shutdown interpretation,
- anomaly handling for unrecognized directory entries,
- recount / recovery-sensitive fallback behavior.

These should only become standalone meso components when they truly have a distinct entry-surface family, a distinct primary Runtime Owner, and a distinct contract that cannot be cleanly attached to another meso.

### 5.7 Candidate Shape for the First Architect Wave

The current working expectation is that the first official Architect artifact should enumerate On-disk Structure Owners first, then Runtime Owners, then a projection matrix between them, then the first candidate meso index, and finally the macro-level lock hierarchy.

At this stage, the exact candidate meso list remains reviewable, but the generation rule above should make the list explicit and reproducible rather than intuition-only.

## 6. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*
1. Re-read `README.md`, `PROTOCOL.md`, `SYSTEM_BLUEPRINT.md`, this live handoff, and the temporary workflow audit `.agents/tmp/20260419-meso_01_round_trace.md`.
2. Treat `pass_01_mount_volume_state` as Creator/Checker/Reviewer accepted. Do not reopen it unless a later integration pass finds a concrete regression.
3. The next `meso_01` lane is the independent meso-level integration Checker pass from `meso_01_mount_volume_state_designer_ktest.md`; keep it distinct from the already accepted Creator-synced pass.
4. Parallel progress may continue on disjoint Designer lanes for already Architected meso components (`meso_02`, `meso_03`, `meso_04`, `meso_06`, `meso_08`), while preserving the no-legacy-`exfat` Creator funnel.
5. Keep using `gpt-5.4` for every delegated subagent and the `codex-asterinas-dev` checker environment with `.agents/tools/checker_lock.sh`.

## 7. Live File Discipline
*Keep this reminder in the active handoff so later main-agents do not split one tenure across many files.*
- **This file is the live handoff for:** April 17 inventory-prior main-agent tenure
- **Update rule:** Update this same file in place for subsequent review, repair, and next-cluster inventory work until ownership intentionally changes.
- **Supersedes / Replaces:** `20260417-1945-mount_inventory_cluster_main_agent_handoff.md`
