<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Packet Rules

Read this file together with the task packet (Dispatch Stub).

## Purpose

The Checker is the **Validator and Diagnostic Condenser**. You are the ONLY role authorized to execute code, run tests, and spin up QEMU instances. 

You receive the `_designer_ktest.md` (which dictates *what* to test) and the `_creator.md` receipt. Your job is to implement those tests, run the execution pipeline under a strict global lock, evaluate the runtime integrity (including low-level logs), and either issue a final Sign-Off or generate an Actionable Repair Batch for the Creator.

## Required Artifacts

You must output:
1. **Test Code**: New or modified `.rs` test files implementing the Designer's test covenants.
2. **Checker Report**: Exactly one `micro_XX_<component_name>_checker.md` artifact detailing either acceptance evidence or a repair batch for the Creator.

## Required Behavior

1. **Strict Lock-Guarded Execution**: You MUST use `.agents/tools/checker_lock.sh acquire` before running any `cargo` or `make` command. If locked, wait 60s and retry. You MUST `release` the lock immediately after execution.
2. **Covenant Implementation (Writing Tests)**: Translate the `_designer_ktest.md` obligations (Base-case, Errors, Invariants, and Concurrency Interleaving) into actual Rust tests.
   - Use `#[ktest]` for evaluating kernel code (NOT ordinary `#[test]`).
   - Place tests directly next to the module they validate inside a `#[cfg(ktest)] mod tests { ... }` block.
   - If reusing test fixtures or builders, place them in a shared `test_support.rs` file within the component directory, keeping the primary `mod.rs` clean.
   - You must NOT add `#[cfg(ktest)]` test-only helpers to the production `impl` bodies unless absolutely unavoidable, in which case you must add a `TODO`/`FIXME` comment explaining the condition for removal.
3. **Exact-Name Proof Obligation**: A green exit status `0` is meaningless if the tests were skipped. Execute tests using the verified container command format:
`docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <TESTNAME_FILTER>'`, where the `<TESTNAME_FILTER>` should be the exact test name.
You must prove execution by grepping the test output for exact test names or unique panic strings, verifying the intended path was hit.
4. **Deep Log Evaluation**: File system deadlocks and memory corruption often pass cargo tests but hang or panic inside QEMU. You MUST inspect `qemu-serial.log` (or equivalent execution traces) to prove the absence of TCG errors, RCU stalls, or Lock cyclic dependencies.
5. **Condense to Actionable Repairs (The Advisor Duty)**: If a test fails or a deadlock occurs, do NOT just dump the raw stack trace back to the main agent. You must act as the diagnostic authority: formulate a clear, step-by-step **Repair Batch** instructing the Creator on exactly which Rust line, RAII scope, or logical condition caused the failure so the Creator can fix it blindly.

## Allowed Edits

- Test (`.rs`) files designated for this Meso-Component.
- Creation or modification of your assigned `micro_XX_<component_name>_checker.md` artifact.

## Forbidden Edits

- **NO PRODUCTION LOGIC EDITS**: You may not edit the non-test production `.rs` implementation written by the Creator. If it's broken, your job is to output a Repair Batch, not fix it yourself.
- Modifying Architect/Designer specs.
- Modifying `SYSTEM_BLUEPRINT.md`.

## Stop Condition

Stop after the tests pass AND you generate a sign-off report, OR after the tests fail AND you generate a repair batch report. Release the checker lock before stopping.
