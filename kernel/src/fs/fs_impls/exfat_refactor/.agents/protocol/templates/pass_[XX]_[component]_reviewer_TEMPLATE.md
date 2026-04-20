<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `{component_name}`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_XX_{component_name}`
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

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C) and against its claimed owner/module boundary.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| `e.g., _read_helper` | Yes | No | `boot-region parser` | Vetoed (No proven Borrow/Yield Hazard, only called once) | REJECT back to Creator for missing census and structural cleanup |
| `e.g., MountStateDispatch` | Yes | Yes | `temporary mount-state facade` | Accepted only with explicit exit plan and real caller evidence | Accepted |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** *(Did every introduced production entity appear in the Creator census? If not, list the omissions.)*
- **Owner / Module Placement:** *(Did any helper sit under the wrong owner, a neutral aggregator, or a catch-all file when it should belong to a narrower module?)* 
- **Temporary Facades / Dead Variants:** *(Did any dispatcher enum, facade, or variant lack a real caller or exit plan?)* 

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
