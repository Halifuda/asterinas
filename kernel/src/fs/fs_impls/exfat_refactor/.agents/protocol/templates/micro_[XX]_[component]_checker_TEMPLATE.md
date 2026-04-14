<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Validation Report / Repair Batch: `{component_name}`

*This artifact is the absolute runtime validation record for the `Checker`. It either acts as a final signature that the Meso-Component successfully executed all Designer KTest covenants safely, or it outputs an Actionable Repair Batch instructing the Creator to fix specific logic flaws / deadlocks.*

## 1. Test Obligations Executed

**Tests Implemented from `_designer_ktest.md` (Using `#[ktest]` and `#[cfg(ktest)] mod tests`):**
- `{test_case_name}`: [e.g., verified base case, written inside `mod tests`]
- `{concurrency_yield_test}`: [e.g., asserted thread starvation during Bio handoff]

## 2. Lock-Guarded Evaluation Result

*Document the results from the Checker Execution Lock stage. Include the explicit `cargo osdk test` command executed in the Docker environment (`codex-asterinas-dev`).*

- **Command Run**: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <TESTNAME>'`
- **Exact-Name Proof**: (Show precise output lines proving the targeted `#[ktest]` actually ran, not just a blind `cargo check`).
- **qemu-serial.log Scan**: (Confirm absence of RCU stalls, TCG panics, or cyclic lock dependencies).

## 3. Conclusion (Accepted OR Repair Batch)

*(Select ONE outcome and delete the other)*

### OUTCOME A: VERIFIED ACCEPTANCE
- **Status:** **PASS**
- All tests succeed. The component's RAII handling matches the Designer's Dynamic Lock Orchestration.

### OUTCOME B: ACTIONABLE REPAIR BATCH FOR CREATOR
- **Status:** **FAIL / DEADLOCK / STALE STATE**
- **Symptom:** *(e.g., "The test `test_bio_yield_stale` panics on line 42 with `PoisonError`. This is because Thread A held the `InodeRwLock` across the yield point, causing Thread B to deadlock.")*
- **Actionable Instruction for Creator:**
   - **Fix 1:** *(e.g., "In `write_at`, before the `await` statement on line 55, you MUST create an explicit block scope to drop `let read_guard` early, satisfying the Designer's yield hazard rule.")*
   - **Fix 2:** *(e.g., "Check the `Bio` result. If it returns `Ok`, you must re-acquire `InodeRwLock` on line 58 and re-validate `inode.size` before proceeding.")*
