<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Pass Validation Report / Repair Batch: `{component_name}`

*This artifact is the absolute runtime validation record for one Checker pass. It either acts as a final signature that the assigned pass executed its Designer KTest covenants safely, or it outputs an Actionable Repair Batch.*

## 1. Pass Identity

**Checker Pass ID:** `pass_XX_{component_name}`
**Pass Kind:** `[Creator-Synced | Meso-Integration]`
**Parent Meso-Component:** `meso_YY_{component_name}`
**Covered Micro-Features:**
- 
**Creator Pass Artifact(s):**
- `path/to/pass_creator.md` *(or `N/A` for a pure integration pass if no single creator receipt applies)*

## 2. Test Obligations Executed

**Tests Implemented from `_designer_ktest.md` (Using `#[ktest]` and `#[cfg(ktest)] mod tests`):**
- `{test_case_name}`: [e.g., verified base case, written inside `mod tests`]
- `{integration_or_concurrency_test}`: [e.g., asserted thread starvation during Bio handoff]

## 3. Lock-Guarded Evaluation Result

*Document the results from the Checker Execution Lock stage. Include the explicit `cargo osdk test` command executed in the Docker environment (`codex-asterinas-dev`).*

- **Reproduce Command**: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <TESTNAME>'`
- **Exact-Name Proof**: (Show precise output lines proving the targeted `#[ktest]` actually ran, not just a blind `cargo check`).
- **qemu-serial.log Scan**: (Confirm absence of RCU stalls, TCG panics, or cyclic lock dependencies).

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
