<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `{component_name}`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_XX_{component_name}`
**Task ID:** `task_{id}`
**Risk Tier:** `[Low | Normal | High]`
**Review Scope:** `[Single stabilized pass | Bounded Meso review wave]`
**Implementation Pass IDs:**
- `pass_XX_{component_name}`
**Parent Meso-Component:** `meso_YY_{component_name}`
**Covered Micro-Features:**
-

- **Naming Conventions:** [e.g., Fixed 3 variables from `a, b, c` to `cluster, offset, length`].
- **Imports:** [e.g., Enforced `StdExternalCrate` grouping policy].
- **Formatting:** [e.g., directly `rustfmt`-aligned inline structs].
- **Doc Comments:** [e.g., Asserted third-person present tense].

| Quality Prior Area | Evidence / Line-Level Finding | Action |
|--------------------|-------------------------------|--------|
| `Error handling` | [e.g., one production `.unwrap()` remained under a locally proven invariant] | [Accepted / Fixed / Rejected] |
| `Visibility` | [e.g., narrowed `pub(crate)` helper to `pub(super)`] | [Accepted / Fixed / Rejected] |
| `Arithmetic / overflow` | [e.g., verified checked math on cluster offsets] | [Accepted / Fixed / Rejected] |
| `Lock / RAII readability` | [e.g., no blocking I/O under forbidden lock scopes] | [Accepted / Fixed / Rejected] |

## 2. Independent Entity Census & Helper Legality Sign-Off

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C, Rule D) and against its claimed owner/module boundary. For full-surface structural-audit packets, this table MUST also include surviving in-scope production `struct`s, `enum`s, carriers, and non-trait helpers even when they predate the current pass.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| `e.g., _read_helper` | Yes | No | `boot-region parser` | Vetoed (No proven Borrow/Yield Hazard, only called once) | REJECT back to Creator for missing census and structural cleanup |
| `e.g., MountStateDispatch` | Yes | Yes | `temporary mount-state facade` | Accepted only with explicit exit plan and real caller evidence | Accepted |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** *(Did every introduced production entity appear in the Creator census? If not, list the omissions.)*
- **Full-Surface Audit Coverage:** *(Required when packeted. Did the artifact disposition every named surviving production `struct`, `enum`, carrier, and non-trait helper in the audited write-set? If not, list the omissions.)*
- **Owner / Module Placement:** *(Did any helper sit under the wrong owner, a neutral aggregator, or a catch-all file when it should belong to a narrower module?)*
- **Temporary Facades / Dead Variants:** *(Did any dispatcher enum, facade, or variant lack a real caller or exit plan?)*

### 2.2 Cleanup Target Closure Verification

*Required when the packet frames the work as structural cleanup. Verify each named cleanup objective independently rather than inferring closure from one visible improvement.*

| Targeted Cleanup Objective | Creator Marked Closed? | Reviewer Judgment | Evidence / Reason | Action |
|----------------------------|------------------------|-------------------|-------------------|--------|
| `e.g., owner-boundary promotion` | `Yes` | `Accepted` | `The old owner-local seam no longer survives on the production path.` | `Accepted` |
| `e.g., temporary error seam` | `No` | `Still open` | `The pass documented the seam but did not yet localize or narrow it.` | `REJECT back to Creator cleanup` |

### 2.3 User-Named Surface Closure

*Required when the user or main agent names concrete symbols, helper families, files, legacy file-local test modules, or legacy test-support paths. Reviewer must reject the pass if any named surface is absent, treated as exempt because it predates the pass, or kept without strong proof.*

| User-Named Surface | Creator Disposition | Reviewer Judgment | If Kept, Is Proof Strong Enough? | Final Action |
|--------------------|---------------------|-------------------|----------------------------------|--------------|
| `e.g., PublishedMountState` | `Kept with invariant proof` | `Accepted / Rejected` | `Yes / No, with reason` | `Accepted / REJECT back to Creator cleanup` |

### 2.4 Carrier Family Review

*Required for any `Target` / `Operation` / `Outcome`, `Snapshot` / `Operation` / `Outcome`, `Validated*` / `Published*` / `State*`, or similar carrier family in scope. Review the family as a whole, not only each type in isolation.*

| Carrier Family | Stable Contract? | Clearer Than Owner Methods + Tuple? | Whole Family Can Be Removed? | Judgment |
|----------------|------------------|-------------------------------------|------------------------------|----------|
| `e.g., MountVolumeStateTarget / Operation / Outcome` | `Yes / No` | `Yes / No` | `Yes / No` | `Accepted / REJECT back to Creator cleanup` |

### 2.5 Validation-Harness Boundary Gate

*Required when validation harness/config code is packeted for review. Any creation, modification, or growth of `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test modules, kernel-mode test-only helpers, `test_support/`, memory-disk fixtures, or other ktest-based validation must be rejected anywhere in the repository. Only explicitly packeted upstream xfstests harness/configuration code outside the filesystem implementation tree may be reviewed.*

| Validation Surface | Current Location | Approved Lane? | Boundary Judgment | Action |
|--------------------|------------------|----------------|-------------------|--------|
| `e.g., xfstests config` | `path/to/harness/file` | `Yes / No` | `Outside fs_impls / violates filesystem-local test ban` | `Accepted / REJECT back to Creator cleanup` |

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** *(Did the temporary struct/facade have an exit-plan comment matching the Designer/Architect notes?)*
- **Edits Made:** *(None, or explain; line-level only.)*

## 4. Edit Scope Classification

- **Reviewer Edit Scope:** *(Choose one: `No edits`, `Line-level non-functional edits only`, `Rejected without edits due structural issues`, or `Line-level edits but final Checker still required`.)*
- **Why This Scope Is Safe:** *(Explain briefly.)*

## 5. Final Verdict

*(Choose ONE)*
- **APPROVED (LINE-LEVEL ONLY; FINAL CHECKER SKIPPABLE)**: The code meets both line-level and structural quality gates. Any reviewer edits are explicitly line-level and non-functional only.
- **REJECTED (STRUCTURAL QUALITY CLEANUP REQUIRED)**: The pass has structural helper / owner-placement / census / module-topology issues. Do not repair them in Reviewer. Route back to a Creator cleanup pass.
- **REJECTED (REQUIRES CHECKER PIPELINE)**: Reviewer edits are still intended to be non-functional, but they are extensive enough that compilation or borrow/RAII validity is no longer certain. Route back through Checker.
