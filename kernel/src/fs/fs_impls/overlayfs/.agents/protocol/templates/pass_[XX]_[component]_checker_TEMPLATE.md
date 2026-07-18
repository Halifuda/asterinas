<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Pass Validation Report / Repair Batch: `{component_name}`

*This artifact is the absolute runtime validation record for one Checker pass. It either acts as a final signature that the assigned pass satisfied its Designer validation contract through the upstream-approved lane, or it outputs an Actionable Repair Batch.*

## 1. Pass Identity

**Checker Pass ID:** `pass_XX_{component_name}`
**Pass Kind:** `[Creator-Synced | Meso-Integration]`
**Parent Meso-Component:** `meso_YY_{component_name}`
**Covered Micro-Features:**
- 
**Creator Pass Artifact(s):**
- `path/to/pass_creator.md` *(or `N/A` for a pure integration pass if no single creator receipt applies)*

## 2. Validation Obligations Executed

**Validation Scenarios Executed from `_designer_validation.md` (using the upstream-approved lane, currently expected to be NixOS xfstests unless superseded by upstream):**
- `{scenario_name}`: [e.g., generic/XXX against mounted filesystem image, with result/notrun/fail receipt path]
- `{integration_or_concurrency_scenario}`: [e.g., xfstests group or explicit upstream suite scenario covering Bio handoff behavior]

## 2.1 Validation Harness Surface Record

*Required whenever Checker created or edited upstream-approved validation harness/config files outside `kernel/src/fs/fs_impls/`. Filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper changes under `kernel/src/fs/fs_impls/` are forbidden for new work.*

- **Touched Validation Harness Surface:** [List touched upstream-approved harness/config files outside `kernel/src/fs/fs_impls/`, or say `None` if Checker reused existing harness only.]
- **Existing Legacy Filesystem-Local Test Surface In Scope:** [Only list packeted pre-existing legacy surfaces if the task explicitly named them for cleanup/audit; otherwise say `None`.]
- **Harness Boundary Justification:** [Explain why any touched harness/config path is outside `kernel/src/fs/fs_impls/` and belongs to the approved validation lane.]
- **Checker Surface Note:** [e.g., `No harness edits`, `Updated xfstests config`, `Possible harness placement concern for Reviewer`]
- **Reviewer Follow-Up Needed:** [Choose `No validation harness surface touched`, `Ordinary post-checker Reviewer gate`, or `Validation harness boundary review required`.]

## 3. Lock-Guarded Evaluation Result

*Document the results from the Checker Execution Lock stage. For compile/build receipts, prefer `.agents/tools/checker_run.sh`; for filesystem behavior validation, record the exact upstream-approved command sequence, currently expected to be NixOS xfstests unless superseded by upstream. If running manually, include the explicit container command executed in `codex-asterinas-dev` or the exact NixOS/QEMU command.*

- **Reproduce Command**: [Exact checker runner, NixOS xfstests, or upstream-approved suite command.]
- **Execution Proof**: [For xfstests, show suite version/config, filesystem type proof, exact generic test IDs or groups executed, and decisive pass/fail/notrun result files. For `cargo-check` or `make-kernel`, show the compile/build command and decisive success/failure lines.]
- **Guest / Suite Log Scan**: [Required for QEMU-backed validation. Inspect preserved `qemu-serial.log`, `qemu.log`, xfstests result files, or equivalent traces for panics, TCG errors, stalls, deadlocks, failures, skips, and notrun classifications. If this pass only ran `cargo-check` or `make-kernel`, say `Not applicable`.]

## 4. Conclusion (Accepted OR Repair Batch)

*(Select ONE outcome and delete the other)*

### OUTCOME A: VERIFIED ACCEPTANCE
- **Status:** **PASS**
- All tests succeed. The assigned pass's RAII handling matches the Designer's Dynamic Lock Orchestration.

### OUTCOME B: ACTIONABLE REPAIR BATCH FOR FOLLOW-UP CREATOR PASS(ES)
- **Status:** **FAIL / DEADLOCK / STALE STATE**
- **Failed Test:** *(e.g., `test_bio_yield_stale`)*
- **Evidence:** *(e.g., "The test panics on line 42 with `PoisonError`, and `qemu-serial.log` shows Thread A held `InodeRwLock` across the yield point, causing Thread B to deadlock.")*
- **Actionable Instruction for Follow-Up Creator Pass(es):**
   - **Fix 1:** *(e.g., "In `write_at`, before the `await` statement on line 55, you MUST create an explicit block scope to drop `let read_guard` early, satisfying the Designer's yield hazard rule.")*
   - **Fix 2:** *(e.g., "Check the `Bio` result. If it returns `Ok`, you must re-acquire `InodeRwLock` on line 58 and re-validate `inode.size` before proceeding.")*
