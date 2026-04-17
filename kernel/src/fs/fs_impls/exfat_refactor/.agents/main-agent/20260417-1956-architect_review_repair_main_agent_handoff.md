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
- None. No meso component was sliced in this session.

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

## 5. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*
1. Re-read `README.md`, `PROTOCOL.md`, `SYSTEM_BLUEPRINT.md`, and this live handoff, then treat the prior layer as stable enough to stop inventory polishing.
2. Prepare the first official Architect dispatch for `macro_00_global_topology` using `protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md` and `protocol/templates/macro_00_global_topology_TEMPLATE.md`, with the packet archived under `subagent-tasks/`.
3. Do not update `SYSTEM_BLUEPRINT.md` beyond Phase 1 scheduling state until the first Architect artifact is actually accepted.

## 6. Live File Discipline
*Keep this reminder in the active handoff so later main-agents do not split one tenure across many files.*
- **This file is the live handoff for:** April 17 inventory-prior main-agent tenure
- **Update rule:** Update this same file in place for subsequent review, repair, and next-cluster inventory work until ownership intentionally changes.
- **Supersedes / Replaces:** `20260417-1945-mount_inventory_cluster_main_agent_handoff.md`
