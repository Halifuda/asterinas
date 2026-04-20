<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: Meso01 Cleanup, Protocol Hardening, and Agent Tools

**Date / Time:** April 20, 2026, 18:15 CST
**Status:** Active Handoff

## 1. Global State Pointer

- Read `SYSTEM_BLUEPRINT.md` first for official pass state.
- `pass_01_mount_volume_state` is still functionally accepted from the previous wave, but it now has a main-agent-identified structural quality debt.
- Do not mark `meso_01_mount_volume_state` fully complete until the independent meso integration pass and the structural cleanup decision are resolved.

## 2. Why Meso01 Needs Cleanup

The first implementation wave produced functionally passing code, but the Reviewer accepted it before the protocol required an independent generated-entity census. The implementation contains too many top-level helpers and temporary facades, especially in `ondisk.rs`.

Key quality concerns:

- `fs.rs` contains Designer-driven dispatcher enums (`MountVolumeStateTarget`, `MountVolumeStateOperation`, `MountVolumeStateOutcome`) that are useful for traceability but should not silently become final-system architecture.
- `MountVolumeStateOperation::Flags` / `MountVolumeStateOutcome::Flags` appear to be dead dispatcher surface because production `FileSystem::flags()` bypasses the dispatcher.
- `fs.rs` has mount helpers (`mount_candidate`, `remount_published`) that are useful but should be owner-local rather than flat top-level helpers.
- `ondisk.rs` is a catch-all file mixing Boot region, VolumeFlags, FAT traversal, root-directory scanning, Allocation Bitmap accounting, Up-case Table loading, byte parsing, checksum helpers, and test diagnostics.
- `DirectoryBootstrap` is especially suspect: it only bundles Allocation Bitmap metadata and Up-case Table metadata found during a root-directory scan. It is not a durable exFAT structure, not a stable runtime owner, and should not survive as a top-level abstraction. Prefer moving those metadata carriers back under Allocation Bitmap / Up-case Table ownership, or at most using a local tuple / narrowly named local record inside a scan helper.

## 3. Recommended Meso01 Cleanup Direction

Open a dedicated non-functional Creator cleanup pass before accepting additional on-disk helper growth.

Suggested cleanup scope:

1. Split `ondisk.rs` into owner-local modules before new implementation passes add more on-disk logic.
2. Keep a small `ondisk/mod.rs` as the public re-export / mount bootstrap surface.
3. Move Boot region parsing, VolumeFlags projection, boot checksum, and geometry validation under a `boot`-owned module.
4. Move FAT sector caching and cluster-chain traversal under a `fat` or `cluster_chain` module.
5. Move Allocation Bitmap metadata parsing and used-cluster counting under an `allocation_bitmap` module.
6. Move Up-case Table metadata parsing, stream loading, and stream checksum under an `upcase` module unless the checksum becomes genuinely shared.
7. Move root-directory mount scan into a narrow bootstrap scanner that returns owner-specific records without preserving a generic `DirectoryBootstrap` final abstraction.
8. Move `#[cfg(ktest)]` diagnostic gate code out of the production helper flow into a test/diagnostic module.
9. Revisit mount-state dispatcher facades in `fs.rs`; either isolate them with explicit exit plans or ask Designer whether the single-interface facade should be revised to a smaller final-system interface set.

This cleanup should be treated as structural quality work. Reviewer should not perform it directly. Dispatch a Creator cleanup pass, then run Checker only if the changes may affect compilation or behavior.

## 4. Protocol Changes Already Made

The protocol and templates were hardened after this finding:

- Creator artifacts now require a full generated-entity census for every introduced production entity.
- Test-only entities must be separately listed instead of silently hidden.
- Reviewer remains post-Checker but now has two required gates:
  - line-level `ASTERINAS_CODE_QUALITY_PRIORS.md` compliance,
  - structural helper / owner-placement compliance.
- Reviewer may directly edit only line-level non-functional issues.
- Structural findings such as missing helper census, owner-placement mismatch, catch-all file growth, dead dispatcher variants, or temporary facades without exit plans must reject back to Creator cleanup.
- Post-review final Checker is skippable only when Reviewer explicitly records line-level non-functional edits only.

Updated files:

- `.agents/PROTOCOL.md`
- `.agents/protocol/CREATOR.md`
- `.agents/protocol/REVIEWER.md`
- `.agents/protocol/CHECKER.md`
- `.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- `.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- `.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`

## 5. New Agent Tools

Two tools were added under `.agents/tools/`.

### Checker Runner

Use `.agents/tools/checker_run.sh` for Checker execution.

Capabilities:

- runs `make kernel` in Docker at `/root/asterinas`,
- runs exact-name `cargo osdk test <FULL_NAME>` in `/root/asterinas/kernel`,
- wraps `checker_lock.sh`,
- archives each `qemu-serial.log` before a later test overwrites it,
- writes a `summary.tsv` and per-command logs.

Example:

```bash
.agents/tools/checker_run.sh pass \
  --component pass_01_mount_volume_state \
  --phase checker \
  --test aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_mount_publishes_root_inode_superblock_and_defaults
```

### Rust Analyzer Code Navigation

Use `.agents/tools/ra_code_nav.py` for symbol-aware code navigation before broad `rg`.

Capabilities:

- `symbols QUERY`
- `file-symbols PATH`
- `definition PATH LINE COL`
- `references PATH LINE COL`
- `implementation PATH LINE COL`
- `hover PATH LINE COL`

Important default:

- Legacy `kernel/src/fs/fs_impls/exfat/**` results are excluded by default to avoid polluting refactor work.
- Use `--include-legacy-exfat` only when a packet explicitly authorizes legacy exFAT inspection.

Examples:

```bash
.agents/tools/ra_code_nav.py --settle-seconds 5 symbols ExfatFs --limit 10
.agents/tools/ra_code_nav.py file-symbols kernel/src/fs/fs_impls/exfat_refactor/fs.rs --limit 30
.agents/tools/ra_code_nav.py references kernel/src/fs/fs_impls/exfat_refactor/fs.rs 43 20 --include-declaration
```

## 6. Next Main-Agent Actions

1. Decide whether to open a dedicated `pass_01_mount_volume_state` structural cleanup Creator pass before any new on-disk implementation pass.
2. If opening cleanup, write a minimal dispatch packet that points to:
   - accepted Creator / Checker / Reviewer artifacts for pass 01,
   - `.agents/PROTOCOL.md`,
   - `.agents/protocol/CREATOR.md`,
   - updated Creator template,
   - `ASTERINAS_CODE_QUALITY_PRIORS.md`,
   - current `fs.rs`, `inode.rs`, `ondisk.rs`, `mod.rs`.
3. Tell the Creator not to mine legacy `kernel/src/fs/fs_impls/exfat/`.
4. Require a full generated-entity census in the cleanup Creator report.
5. Route the cleanup to Checker if module movement or any behavior-adjacent edit makes compile/runtime validity uncertain.
6. Route final cleaned code to Reviewer under the new line-level + structural gate.

## 7. Session Update: Pass 01 Cleanup Launch

- Decision made: open a dedicated Creator cleanup pass for the already accepted `pass_01_mount_volume_state` implementation surface only.
- Scope boundary: this is **not** a reopening of all `meso_01_mount_volume_state` work. The cleanup is limited to the Rust code currently owned by `pass_01_mount_volume_state`; later unfinished `meso_01` work remains out of scope.
- Archived dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_creator_dispatch.md`.
- Pass tracker updated in `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md` with `pass_01_mount_volume_state_cleanup_01`.
- Namespace decision: prefer owner-local top-level modules under `kernel/src/fs/fs_impls/exfat_refactor/`; do not create a deep `ondisk/` submodule tree unless later file growth proves it necessary.
- `DirectoryBootstrap` is no longer an open design question. It should not survive this cleanup because it only grouped Allocation Bitmap metadata and Up-case metadata that belong to those owner-local types.
- Cleanup expectation: preserve the already accepted pass-scoped behavior and checker targets while paying down structural debt in `ondisk.rs` and related pass-01-owned helper placement.

## 8. Revised Next Main-Agent Actions

1. Delegate `pass_01_mount_volume_state_cleanup_01` to a Creator lane using the archived dispatch stub.
2. Require the Creator result to keep behavior stable while splitting or relocating helpers only within the accepted `pass_01` surface.
3. Send the cleanup result to Checker if module moves, symbol relocation, or test-adjacent edits make compile/runtime confidence uncertain.
4. Send the stabilized cleanup result to Reviewer under the dual line-level + structural gate.
5. Keep `meso_01_mount_volume_state` integration work separate; do not fold unfinished later `meso_01` work into this cleanup lane.

## 9. Deferred Structural Doubts To Preserve

- User guidance for this wave distinguishes three cleanup buckets explicitly: (1) entities that should not exist at all, (2) entities that should exist but in their own file, and (3) entities that may exist but are currently placed under the wrong owner/module boundary.
- The current `pass_01_mount_volume_state_cleanup_01` launch hardens only a subset of those findings so the pass does not widen uncontrollably.
- In particular, several `*Record` / `*State` carriers still have unresolved existence questions beyond simple relocation. These doubts are preserved here so later main-agent work does not accidentally treat them as already accepted final-system shapes.
- Suspect-but-deferred examples include `AllocationBitmapRecord`, `UpcaseRecord`, `VolumeAnomalyState`, `ValidatedMount`, `AllocatorState`, and `PublishedMountState` if later cleanup shows they are only transit carriers rather than stable owner-local abstractions.
- Decision for this wave: record those doubts, but do not require the current cleanup pass to eliminate every `Record` / `State` carrier. The current pass stays focused on the already-agreed structural debt: removing `DirectoryBootstrap`, shrinking `ondisk.rs`, splitting owner-local modules, and trimming clearly dead or temporary dispatcher surface inside `pass_01` only.
- Re-evaluate the remaining `Record` / `State` carriers after the scoped `pass_01` cleanup stabilizes; do not silently bless them as final architecture in the meantime.

## 10. Cleanup Routing Rule For This Wave

- Execution order for this cleanup wave is intentionally `Creator -> Reviewer` first, not `Creator -> Checker`.
- Reason: this wave is primarily structural cleanup inside the already accepted `pass_01` surface. We want Reviewer to confirm that the structural debt is actually paid down before opening a Checker repair lane for any compile/test fallout.
- Checker is therefore deferred until cleanup structure is confirmed. After Reviewer either approves or returns only line-level quality edits, the main agent may open a Checker lane to repair any test breakage introduced by the cleanup and then run the required build / exact-name ktests.
- If Reviewer instead finds unresolved structural debt, route back to another Creator cleanup pass without spending Checker time yet.

## 11. Session Update: Tightened Packet + Creator Launch

- Tightened `pass_01_mount_volume_state_cleanup_01` so the packet now encodes the three agreed cleanup buckets explicitly instead of only saying "split `ondisk.rs`" in generic terms.
- The tightened packet now names: (1) entities that should not survive at all (`DirectoryBootstrap`, and dispatcher-only surface if still dead), (2) entities that should move into owner-local top-level files, and (3) entities that may remain but must leave the neutral catch-all placement.
- The packet also preserves the bounded-wave rule: some `*Record` / `*State` carriers remain structurally suspect, but this pass does not widen into a full existence re-judgment of every such carrier.
- Creator cleanup lane launched with agent `Ptolemy` using `gpt-5.4`.
- Current router plan: wait for the Creator artifact, inspect census/write-set/scope compliance, then archive a Reviewer dispatch for the same cleanup pass before any Checker work begins.

## 12. Session Update: Reviewer Packet Pre-Staged

- While `Ptolemy` is still running, the Reviewer dispatch for the same cleanup pass has been pre-staged at `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_reviewer_dispatch.md`.
- The Reviewer packet points to both the original `pass_01` receipts and the pending cleanup Creator artifact so Reviewer can judge whether the structural debt was actually retired rather than merely moved around.
- The Reviewer packet also records the wave-specific routing rule: confirm cleanup structure first, and only after that open Checker for compile/test fallout.

## 13. Session Update: Creator Cleanup Result Accepted For Review Routing

- `Ptolemy` completed `pass_01_mount_volume_state_cleanup_01` and produced `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_creator.md`.
- Main-agent spot check confirms the packet's most explicit requirements were met before review routing:
  - `DirectoryBootstrap` is gone from `kernel/src/fs/fs_impls/exfat_refactor/`.
  - `ondisk.rs` is reduced to a thin compatibility re-export surface.
  - owner-local top-level files now exist for boot/bootstrap, FAT traversal, Allocation Bitmap logic, Up-case logic, and `#[cfg(ktest)]` diagnostics.
  - dispatcher-only `RootInode` / `SuperBlock` / `Flags` branches were removed from `mount_volume_state`.
- Structural caveat preserved: the result is not treated as final acceptance yet. The user already signaled dissatisfaction with parts of the shape, and several `*Record` / `*State` carriers remain intentionally deferred rather than resolved in this wave.
- Routing decision: send the result to Reviewer now under the pre-staged cleanup-review packet. Defer Checker until Reviewer confirms the structural cleanup is complete enough to justify compile/test fallout repair.

## 14. Session Update: Reviewer Rejection Routed Into Cleanup 02

- Reviewer `Gibbs` rejected `pass_01_mount_volume_state_cleanup_01` for one focused structural issue: `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` still imports mount-state carriers through `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`, so the old catch-all boundary remains live on the production path.
- Main-agent accepts that rejection as scoped and actionable. No Checker work should open yet because the debt is still structural, not test fallout.
- Archived follow-up dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_creator_dispatch.md`.
- Cleanup 02 scope is intentionally narrow:
  - retire `ondisk.rs` from the production `fs.rs` import path,
  - keep behavior unchanged,
  - avoid widening into a fresh pass over all deferred `*Record` / `*State` doubts.
- Current router plan: launch a new Creator cleanup lane on the focused Reviewer batch, then return to Reviewer again before any Checker lane opens.

## 15. Session Update: Entity-Audit Gap Made Explicit

- User correctly pointed out a protocol-execution gap: the current Creator / Reviewer templates speak in terms of **introduced** entities, which is narrower than a full audit of all surviving production entities in the write-set.
- The stricter interpretation for the current wave is now recorded here: Reviewer for `pass_01_mount_volume_state_cleanup_02` must inspect the full surviving production entity surface in the assigned write-set, not only newly introduced entities.
- Practical implication: exposed production free-helper families are now an explicit review target even when they were merely retained rather than newly introduced in `cleanup_02`.
- This does not yet rewrite the top-level protocol text; it is a wave-local enforcement tightening captured in the cleanup-02 Reviewer packet and this live handoff so the current loop does not silently pass remaining naked helper debt.

## 16. Session Update: Cleanup Confirmed, Checker Opened By User Request

- Reviewer `James` approved `pass_01_mount_volume_state_cleanup_02` with line-level non-functional import-hygiene edits only and explicitly accepted the surviving owner-local helper families under the wave-local full-entity audit.
- Under protocol rule 26 the final Checker would be skippable for the Reviewer edits alone, but the user had already asked to open Checker after cleanup confirmation so test fallout can be repaired and the pass can be re-validated.
- Archived Checker dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_checker_dispatch.md`.
- Checker scope for this wave:
  - run full compile via `.agents/tools/checker_run.sh make-kernel`,
  - run the ten exact-name `mount_volume_state_*` ktests again,
  - repair only checker-owned `#[cfg(ktest)]` fallout if needed,
  - route any production failure back as a Creator repair batch rather than patching production logic in Checker.

## 17. Session Update: User Reopened Helper-Family Cleanup Despite Reviewer Approval

- Checker `Halley` has already completed and produced a compile-failure repair batch in `pass_01_mount_volume_state_cleanup_02_checker.md`; that result is preserved and not discarded.
- Separately, the user explicitly asked that the helper issue continue even after the stricter Reviewer approval. Main-agent therefore reopens pass-01 again under a new Creator cleanup lane focused on surviving production free-helper families.
- Archived dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_creator_dispatch.md`.
- Cleanup 03 scope:
  - re-judge surviving naked helper families in `boot.rs`, `fat.rs`, `bitmap.rs`, and `upcase.rs`,
  - absorb or narrow them when possible,
  - require explicit rationale for any helper left exposed,
  - optionally fold in the compile fixes from the Checker repair batch if doing so stays within the same local write-set and does not widen scope.
- This reopen is user-driven even though `cleanup_02` Reviewer approved the current shape. The approval is no longer treated as final closure on helper quality.

## 18. Session Update: Cleanup 03 Creator Returned

- `Turing` completed `pass_01_mount_volume_state_cleanup_03` and reports that the surviving production helper families in `boot.rs`, `fat.rs`, `bitmap.rs`, and `upcase.rs` were absorbed into owner-local impl blocks.
- Spot check confirms the intended direction:
  - `boot.rs`, `fat.rs`, `bitmap.rs`, and `upcase.rs` now expose impl-heavy owner-local surfaces instead of module-level production helper families.
  - `fs.rs` mount bootstrap now routes through `boot::ValidatedMount::load(...)`.
  - the local compile fixes previously reported by Checker (`VmIo` scope and root-directory visitor return shape) were folded into the same owner-local cleanup.
- Archived Reviewer dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_reviewer_dispatch.md`.
- Current router plan: send cleanup 03 to Reviewer now to confirm the helper-family cleanup actually holds under static review before deciding whether to reopen Checker on the new code state.

## 19. Session Update: Cleanup 03 Reviewer Approved, Checker Reopened

- Reviewer `Copernicus` approved `pass_01_mount_volume_state_cleanup_03` and wrote `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_reviewer.md`.
- Reviewer made one line-level `#[cfg(ktest)]` import fix in `diagnostics.rs` (`use ostd::mm::VmIo;`) and found the production helper-family cleanup structurally acceptable.
- Because cleanup 03 folded compile fixes from the earlier Checker repair batch and Reviewer touched a ktest-only import, the main agent reopened Checker rather than skipping final validation.
- Archived Checker dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_checker_dispatch.md`.
- Checker must run full compile and the ten exact-name `mount_volume_state_*` ktests, preserving exact-name proof and serial-log evidence.

## 20. Session Update: Cleanup 03 Checker Passed; Cleanup 04 Opened For Remaining Structural Concerns

- Checker `Pascal` passed `pass_01_mount_volume_state_cleanup_03`: full `make kernel` succeeded, all ten exact-name `mount_volume_state_*` ktests passed, and serial logs were archived with no panic/deadlock evidence.
- User then identified two remaining structural concerns:
  - repeated `read_le_u16` / `read_le_u32` / `read_le_u64` wrappers remain scattered across production and ktest-only code even though they mostly wrap `from_le_bytes(...)`,
  - `diagnostics.rs` is a poor long-term name/layout for ktest-only support and should not become a single flat file containing all future tests/helpers.
- Main-agent agrees these are legitimate structural review targets and opened a new Reviewer-first loop instead of accepting the cleanup as final.
- Archived Reviewer dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer_dispatch.md`.
- Reviewer must decide whether endian helpers should be removed/centralized and what test-only module layout should replace the current flat `diagnostics.rs` shape; if cleanup is required, Reviewer should reject with explicit Creator instructions.

## 21. Session Update: Cleanup 04 Reviewer Rejected, Creator Queued

- Reviewer `Herschel` rejected `pass_01_mount_volume_state_cleanup_04` for structural cleanup.
- Required Creator actions from Reviewer:
  - delete duplicated `read_le_u16` / `read_le_u32` / `read_le_u64` wrappers and inline direct fixed-width `from_le_bytes(...)` at call sites,
  - replace flat `diagnostics.rs` with a dedicated `#[cfg(ktest)]` test-support hierarchy,
  - enumerate the moved / introduced test-only helpers explicitly in the Creator report.
- Archived Creator dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_creator_dispatch.md`.
- Current router plan: launch Creator on this focused structural cleanup, then return through Reviewer before any Checker run.

## 22. Session Update: Protocol/Skill Rules Hardened, Cleanup 04 Creator Returned

- Protocol and skill docs were updated to make the recently learned structural-quality expectations durable:
  - cleanup passes may explicitly re-audit surviving entities,
  - naked helper families are an explicit structural-review surface,
  - thin `read_le_*` wrappers over `from_le_bytes(...)` are presumed unwanted,
  - non-trivial `#[cfg(ktest)]` support should live under a dedicated test-support hierarchy.
- Updated files:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
  - `/home/halifuda/.codex/skills/exfat-main-agent/SKILL.md`
  - `/home/halifuda/.codex/skills/exfat-subagent-workflow/SKILL.md`
- `Descartes` then completed `pass_01_mount_volume_state_cleanup_04`.
- Spot check at that time confirms:
  - `rg "read_le_" kernel/src/fs/fs_impls/exfat_refactor --glob '*.rs'` is empty,
  - the flat `diagnostics.rs` file is gone,
  - a dedicated `test_support/` hierarchy now exists,
  - `ondisk.rs` still remained as a ktest-only compatibility shim re-exporting from `test_support` before the later follow-up deletion recorded below.
- Archived follow-up Reviewer dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer_followup_dispatch.md`.
- Current router plan: send cleanup 04 through follow-up Reviewer now, then reopen Checker to inspect any remaining test-support utility extraction opportunities and rerun compile + existing exact-name ktests if the review passes.

## 23. Session Update: Follow-Up Reviewer Approved, But Shim Retention Rejected By User

- `Hubble` completed the follow-up review in `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer_followup.md`.
- Reviewer confirmed the requested cleanup goals from `cleanup_04` were met: duplicated thin endian wrappers are gone, `diagnostics.rs` is gone, and the ktest-only surface now lives under `test_support/`.
- However, Reviewer still treated the remaining `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` file as an acceptable ktest-only compatibility shim.
- User rejected that retention: even a ktest-only catch-all shim is still redundant path indirection once direct owner/test-support paths already exist.

## 24. Session Update: Main-Agent Removed The Obsolete `ondisk.rs` Shim Directly

- Main-agent inspected the live tree and found `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` had become an unreferenced pure re-export seam. `rg -n "ondisk::|super::super::ondisk|mod ondisk" kernel/src/fs/fs_impls/exfat_refactor --glob '*.rs'` found only the `#[cfg(ktest)] mod ondisk;` declaration in `mod.rs`.
- Because the shim no longer carried logic, no longer had call sites, and only existed to preserve a redundant name layer, main-agent removed it directly instead of reopening another Creator packet just to delete one dead compatibility file.
- Applied edits:
  - deleted `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
  - removed `#[cfg(ktest)] mod ondisk;` from `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- This is treated as bounded structural cleanup inside `pass_01_mount_volume_state` only, not as a reopening of later unfinished `meso_01_mount_volume_state` work.

## 25. Session Update: Checker Had Not Yet Covered The Post-`cleanup_04` Tree

- User asked whether the Checker had already performed the requested post-cleanup work. Answer: **not yet for the current post-`cleanup_04` tree**.
- Existing Checker evidence only covers the earlier `cleanup_03` tree (`pass_01_mount_volume_state_cleanup_03_checker.md`), before `diagnostics.rs` removal / `test_support/` split / final shim deletion settled.
- Main-agent therefore opens a final Checker pass for `cleanup_04` rather than falsely treating the earlier receipt as sufficient.

## 26. Session Update: Checker Rules And Packet Updated For Test-Support Topology

- While preparing the final Checker lane, main-agent found `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md` still carried an older suggestion to place reused helpers in a flat `test_support.rs` file.
- That wording now conflicts with the wave-local accepted rule that non-trivial `#[cfg(ktest)]` support should live under a dedicated `test_support/` hierarchy split by concern.
- The Checker rule was updated accordingly so later lanes do not regress toward a new flat helper bucket.
- Archived final Checker dispatch: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_checker_dispatch.md`.
- Checker scope is now explicit:
  - inspect `test_support/` for any remaining utility-bucket shape and split only if still needed,
  - repair only checker-owned `#[cfg(ktest)]` fallout if current code no longer compiles/tests after shim removal,
  - rerun `make kernel` plus the ten exact-name `mount_volume_state_*` ktests on the current tree.

## 27. Session Update: Final Checker Passed On The Post-Cleanup Tree

- `Pascal` completed the final Checker lane and produced `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_checker.md`.
- Full compile passed on the current tree after shim removal, and all ten exact-name `mount_volume_state_*` ktests passed. Archived receipts live under `kernel/src/fs/fs_impls/exfat_refactor/.agents/checker-runs/pass_01_mount_volume_state_cleanup_04/manual-20260420-232626/`.
- Checker found only checker-owned `#[cfg(ktest)]` fallout:
  - `test_support/mod.rs` needed a local wrapper instead of a too-wide re-export for `diagnose_invalid_on_disk_layout_gate`,
  - `test_support/boot_region.rs`, `test_support/root_directory.rs`, `test_support/bitmap.rs`, and `test_support/upcase.rs` needed local `VmIo` imports for `read_bytes`.
- No production logic changed in the Checker pass.
- Checker explicitly confirmed that no further utility-bucket split is needed inside `test_support/`: the current `mount_diagnostics`, `boot_region`, `root_directory`, `bitmap`, and `upcase` split is already the right boundary for this wave.

## 28. Current Closure State For `pass_01` Cleanup

- `pass_01_mount_volume_state` remains only **functionally accepted** at the broader meso level because later `meso_01_mount_volume_state` work is still unfinished.
- But the scoped `pass_01` structural cleanup wave requested by the user is now closed:
  - `DirectoryBootstrap` removed,
  - owner-local top-level modules split out,
  - naked helper families absorbed,
  - duplicated thin endian wrappers removed,
  - flat `diagnostics.rs` replaced by a dedicated `test_support/` hierarchy,
  - obsolete ktest-only `ondisk.rs` shim deleted,
  - final Checker passed on the current tree.
