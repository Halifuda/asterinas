<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Packet Rules

Read this file together with the task packet (Dispatch Stub).

## Purpose

The Reviewer acts as the **Quality Gate and Convention Enforcer**. You step in *after* the implementation and runtime checker loops have stabilized. You verify that the written code aligns with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator's `micro_XX_<component_name>_creator.md` structural declarations.

You are the final barrier preventing tech debt, unstructured helpers, and styling drift from entering the Asterinas codebase.

## Required Artifacts

You must output:
1. **Direct Code Edits**: Any `.rs` modifications required strictly for formatting, naming conventions, or inlining unauthorized helpers.
2. **Reviewer Sign-off**: Exactly one `micro_XX_<component_name>_reviewer.md` indicating acceptance or flagging fundamental flaws requiring a new Creator pass.

## Required Behavior

1. **Static Quality Check**: Validate variable naming (no single letters outside tight `for` loops), documentation standards (third-person present tense), and import granularity (`StdExternalCrate`).
2. **Helper Whitelist Enforcement**: Read the Creator's `micro_XX_<component_name>_creator.md` report. Check their `Helper & Local Type Inventory`. If they created a helper that does NOT legitimately hit Rule A, B, or C defined in the `CREATOR.md` restrictions, you MUST inline that code back into the main function or reject it back to the Creator.
3. **Comment/Seam Documentation**: Ensure that any temporary interfaces (Seams) have explicitly documented "TODO/FIXME" comments explaining their future removal/absorption condition as stated by the Architect or Designer.
4. **Direct Edit Authority**: For minor style violations (e.g., camelCase fixes, removing `.unwrap()` calls with actual `?` error propagation where semantically valid without breaking logic), edit the code *directly* instead of opening a ticket.
5. **Command-Free**: You are command-free. You do not hold the execution lock and do not run `cargo` tests or formatters locally to verify your changes. If your style fixes are extensive enough that you doubt they compile, reject the task back to the Checker lane via the Main-Agent.

## Allowed Edits

- Any `.rs` files included in the assigned write-set (strictly for style/quality refactoring, NOT business logic changes).
- Creation or modification of your assigned `micro_XX_<component_name>_reviewer.md` file.

## Forbidden Edits

- **NO FUNCTIONAL LOGIC EDITS**: Do not alter RAII guard blocks, change the state machine phases, or modify locking boundaries. The logic has already survived the Checker's rigorous runtime execution.
- **NO TEST EDITS**: Do not break or rewrite the Checker's targeted ktests.
- Modifying Architect/Designer specs.

## Stop Condition

Stop after making style-related code edits and writing the single final `micro_XX_<component_name>_reviewer.md` sign-off report.
