<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Final Quality Sign-Off: `{component_name}`

*This artifact forms the final Reviewer quality gate. Checking if the Creator's `micro_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Static Quality Enforcement Log

**Target Component:** `{component_name}`

- **Naming Conventions:** [e.g., Fixed 3 variables from `a, b, c` to `cluster, offset, length`].
- **Imports:** [e.g., Enforced `StdExternalCrate` grouping policy].
- **Formatting:** [e.g., directly `rustfmt`-aligned inline structs].
- **Doc Comments:** [e.g., Asserted third-person present tense].

## 2. Creator Helper Legality Sign-Off

*You must cross-reference the Creator's `micro_XX_creator.md` report. Evaluate the Helper & Local Type inventory against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C).*

| Handled Symbol | Whitelist Judgment | Action Taken (Accepted / Rejected / Inlined) |
|----------------|--------------------|----------------------------------------------|
| `e.g., _read_helper` | Vetoed (No proven Borrow/Yield Hazard, only called once) | INLINED back into the main function body to remove tech debt. |

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** *(Did the temporary struct/facade have an exit-plan comment matching the Designer/Architect notes?)*
- **Edits Made:** *(None, or explain)*

## 4. Final Verdict

*(Choose ONE)*
- **APPROVED**: The code meets all static quality constraints and contains zero illegal entities. Ready for final Main-Agent integration.
- **REJECTED (REQUIRES CHECKER PIPELINE)**: Edits were substantial enough that they might break compiler/borrow checker RAII scopes. Do not merge. Route back through the Checker lane.
