<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `exfat_refactor` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.

Use the surrounding documents as follows:
- `README.md`: workspace map and project framing
- `$exfat-main-agent`: preferred Codex entry point for main-agent tasks
- `$exfat-subagent-workflow`: preferred Codex entry point for delegated subagent tasks
- `protocol/`: source-text role rules mirrored by the subagent skill

## 0. Core Terms

- **Final Owner**: The stable finished-system owner (VFS trait carrier, On-disk Structure Owner, daemon process, record type).
- **On-disk Structure Owner**: A Final Owner for one concrete durable exFAT structure or state machine, such as the Boot region, Allocation Bitmap, FAT, Up-case Table, directory-entry set, Stream Extension, or volume-label / volume-GUID entry. Use this full term; do not shorten it to an ambiguous generic phrase.
- **Macro-Owner**: The large-scale architectural entities that belong to the final-owner concept, including VFS trait carriers and On-disk Structure Owners (e.g., `ExfatFs`, `ExfatInode`, `AllocationBitmap`, `UpcaseTable`, `Fat`).
- **Meso-Component**: Explicit interfaces and primary structures mapped under a Macro-Owner (e.g., `write_at`, `resize`).
- **Micro-Feature**: The specific functional details derived from prior knowledge (e.g., file write zero-fill gaps, allocation cluster counting, timestamp updates).
- **Creator Pass**: A main-agent-defined implementation slice that sits between a Meso-Component and its Micro-Features. Each Creator Pass names exactly one parent meso-component and one explicit covered-micro set.
- **Checker Pass**: A test/validation slice. For implementation work it MUST mirror the Creator Pass exactly; meso-level integration testing is scheduled as an independent Checker-only pass with its own covered-micro set.
- **Global Lock Topology**: The absolute static hierarchy and holding states of synchronization primitives in the system.
- **Information Funnel & Dispatch Stubs**: Heavy priors are internalized by higher roles (Architect). Lower roles (Creator) receive minimal context via Dispatch Stubs to prevent architectural overreach.

## 1. Scheduler Rules

1. Every agent must obey the repository-level `AGENTS.md`. No `unsafe` in `kernel/src/fs/fs_impls/exfat_refactor/`.
2. The main agent is the only scheduler. Only the main agent changes official component state or the `SYSTEM_BLUEPRINT.md`.
3. No component enters implementation before its Architect handoff and Designer artifacts exist. Designer artifacts stay meso-scoped: a comprehensive logic/lock spec (`_designer_spec.md`) plus `_ktest.md`.
4. **Main-Agent-Owned Pass Slicing**: The main agent MUST decide Creator Pass boundaries. Architects and Designers must stay exhaustive at the meso level and must not pre-slice implementation passes. Every Creator, Checker, and Reviewer packet MUST declare the parent meso-component and covered micro-features explicitly.
5. **Strict Acceptance via Templates**: Main-Agent acceptance is purely structural, not logical. Subagent artifacts MUST fully populate their corresponding `protocol/templates/...` structures. If a template section (e.g., Designer integration-test obligations, or Creator covered-micro declaration) is omitted or conceptually empty, the Main-Agent must reject it outright for protocol violation.
6. Components may depend only on accepted components or stable pre-existing kernel interfaces.
7. The `exfat` module remains the active registered filesystem until explicitly scheduled for takeover.
8. **Creator/Checker Pass Synchronization**: Every Creator Pass MUST have a matching Checker Pass with the same parent meso-component and the same covered-micro set. The main agent must not widen or narrow that Checker Pass relative to the Creator Pass.
9. **Independent Meso Integration Passes**: The meso-level integration tests declared in `_designer_ktest.md` are NOT folded into Creator-synced Checker passes. They are scheduled as separate Checker-owned passes after the relevant implementation passes exist.
10. Test authoring is Checker-owned by default. Creators should ignore test-writing unless the packet overrides this.
11. Role command authority: Main agent, Architect, Designer, Reviewer must not run kernel build/test commands. Creators are command-free. Checkers own runtime verification commands.
12. **Code Navigation Tooling**: All roles may use `.agents/tools/ra_code_nav.py` for read-only rust-analyzer navigation when packet scope permits inspecting Asterinas Rust code. This tool is for symbol-aware navigation (`symbols`, `file-symbols`, `definition`, `references`, `implementation`, `hover`); it does not authorize reading files outside the packet's allowed context and it does not replace role artifacts or design reasoning.
13. **Checker execution is lock-guarded**:
   - Preferred execution wrapper: `.agents/tools/checker_run.sh`, which acquires/releases the checker lock, runs Docker commands in `/root/asterinas`, and archives each test's `qemu-serial.log` before it is overwritten.
   - Low-level lock helper: before manual execution (build/test/QEMU), Checker MUST use `tools/checker_lock.sh acquire`.
   - If locked, Checker waits quietly and retries (minimum `60` seconds).
   - After completion, Checker MUST release the lock via `tools/checker_lock.sh release`, or use `checker_run.sh` so release is handled by the wrapper.
   - Only the main agent may clear a stale lock.
14. **Exact-Name Proof Obligation**: Filtered verification commands must prove they targeted intended tests. A green exit status alone is insufficient when `cargo osdk test <filter>` is used. The Checker must record an exact, uniquely justified test-path suffix or output that explicitly names executed tests.
15. **Evaluating qemu-serial.log**: Checkers MUST examine `qemu-serial.log` (or other execution traces) for panics, TCG errors, or deadlocks. Exit codes are not enough. When multiple ktests run in one Checker pass, use `checker_run.sh` or otherwise preserve each test's serial log before the next run overwrites it.
16. **Integrated Repair & Blind Passthrough**: Checkers analyze failures directly and condense them into actionable repair batches. The main agent acts strictly as a router here; it MUST NOT reinterpret tests or rewrite the diagnostic. Creator-synced Checker failures route back to the same Creator Pass. Independent integration-pass failures route to one or more reopened Creator Passes or escalate upward, but the Checker's reproduce command, failed test, and evidence must be preserved verbatim.
17. **Escalation Path (Max Retries = 5)**: If a Creator-Checker repair loop cycles 5 times without producing a passing exact-name test receipt, the main agent MUST halt the loop and package the impasse to escalate upwards (e.g., sending the deadlock/crash history back to DESIGNER to re-evaluate lock constraints).
18. Command-free work (Architect, Designer, Reviewer, Creator passes) should fill parallel lanes whenever dependency graphs and write-sets allow, even while the checker execution lock is held.
19. If a delegated command-free lane stalls, the main agent should repair and re-delegate instead of absorbing the work into the main thread.
20. **Strict Information Funnel**: Packets MUST be saved in `subagent-tasks/<component-id>/` and MUST use the `protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md`. The main agent MUST NOT write design summaries or architectural hints in the packet. The packet is purely a pointer route (File Paths) to the input files and the output template.
    - **To Architect**: Inputs = `priors/Microsoft-exFAT-spec.md`, `priors/linux-exFAT-implementation-summary.md`, and the relevant Asterinas priors.
    - **To Designer**: Inputs = Architect topology and local component context.
    - **To Creator**: Inputs = Designer's contract spec, the main-agent-selected covered-micro set, and `priors/ASTERINAS_CODE_QUALITY_PRIORS.md`, plus only the stable pre-existing kernel interfaces strictly required to typecheck against Asterinas. NEVER supply heavy exFAT specs, Linux code, or legacy `kernel/src/fs/fs_impls/exfat/` implementation references to Creator.
    - **To Checker (Creator-Synced Pass)**: Inputs = Designer `_designer_ktest.md`, the matching Creator Pass report, and the pass write-set/code paths.
    - **To Checker (Meso Integration Pass)**: Inputs = Designer `_designer_ktest.md`, the accepted Creator Pass reports covering the target micro-features, and the pass write-set/code paths.
21. If local Asterinas interfaces force a divergence from Microsoft or Linux behavior, the Architect or Designer artifact must record that explicitly.
22. **Creator Entity Census Is Mandatory**: Every Creator artifact MUST include a complete census of newly introduced production entities in the assigned write-set. This includes every introduced `struct`, `enum`, local type alias, module, and non-trait helper function. Trait-required methods may be grouped explicitly under their impl block instead of listed one-by-one. Test-only entities MUST appear in a separate subsection and are not exempt from reporting. If an introduced entity is missing from the census, the Main-Agent MUST reject the Creator artifact for protocol violation.
23. **Cleanup Passes May Re-Audit Surviving Entities**: For ordinary feature work, the mandatory census centers on introduced entities. But when a pass is explicitly framed as structural cleanup, the main agent may require re-auditing surviving entities already present in the write-set. In those passes, Reviewer and Creator must not treat "this helper predates the pass" as an automatic exemption.
24. **Reviewer Runs After Checker, With Two Distinct Gates**: Reviewer remains the post-checker static gate. Reviewer MUST enforce both (a) line-level `ASTERINAS_CODE_QUALITY_PRIORS.md` compliance and (b) structural helper / owner-placement quality. Passing one gate does not excuse failure in the other.
25. **Reviewer Must Treat Naked Helper Families As Structural Surface**: Reviewer must not only inspect single helpers in isolation. Clusters of naked free helper functions under one owner module are themselves a structural-quality target. If that family should instead live on an owner-local type / impl boundary, Reviewer MUST reject it back to Creator cleanup.
26. **Thin Endian Wrappers Are Presumed Unwanted**: Tiny wrappers whose only job is to call fixed-width `from_le_bytes(...)` are presumed unnecessary. Keep them only when they carry a real owner-local invariant, validation boundary, or error-translation contract. Otherwise inline the decode at call sites instead of preserving repeated `read_le_*` helper families.
27. **Test-Only Support Must Use A Dedicated Hierarchy**: `#[cfg(ktest)]` support code must not accumulate in a flat catch-all sibling file as the surface grows. When test-only helpers become non-trivial, they should move under a dedicated test-support subtree (for example `test_support/`) with files split by concern, keeping production ownership flow and test-only support clearly separated.
28. **Reviewer Direct Edits Are Narrow**: Reviewer may directly edit only line-level, non-functional issues that preserve behavior and module topology, such as naming, import grouping, formatting, narrow visibility tightening, comment/seam wording, or locally proven non-functional cleanup like replacing an unnecessary `.unwrap()` with equivalent error propagation. Reviewer MUST NOT perform broad structural refactors, helper relocation across owner boundaries, module splitting, or topology-changing cleanups inside the Reviewer pass.
29. **Structural Quality Findings Reopen Creator, Not Reviewer**: If Reviewer finds helper-census omissions, owner-placement mismatches, catch-all file growth, dead dispatcher variants, temporary facades without exit plans, duplicated thin endian wrappers, scaling-poor flat test-only helper namespaces, or other structural-quality issues, Reviewer MUST reject the pass back to a dedicated Creator cleanup pass instead of directly rewriting the structure. That cleanup pass re-enters Checker only if the resulting edits may affect compilation or runtime behavior.
30. **Post-Review Final Checker Is Conditional**: The main agent may skip the post-review final Checker only if Reviewer explicitly records that all edits were line-level and non-functional only, no tests or checker-owned surfaces changed, and no structural cleanup was performed. Otherwise the pass MUST return to Checker.
31. **Continuous Main-Agent Handoff**: The active main-agent thread MUST maintain exactly one live handoff record in `.agents/main-agent/` using `protocol/templates/YYYYMMDD-HHMM-nickname-summary_main_agent_handoff_TEMPLATE.md`. During a single main-agent tenure or continuous work span, the main agent MUST update that same file in place instead of creating a new handoff file for each micro-session, review, or repair step. A new handoff file should appear only when a genuinely new main-agent tenure starts or when ownership is intentionally rolled forward to a new live note. Every material scheduling action, pass-slicing decision, template acceptance/rejection, and escalation MUST be reflected in the live file during the active wave. Before closing a session, the handoff MUST conclude with explicit next-main-agent tasks.
32. **Artifact Directory Structure**: Subagent produced artifacts (specs, code analysis, checker reports) MUST be placed under `.agents/components/<component-id>/`. Mixed directories are not allowed.

## 2. Role Ownership

- **Main agent**: Owns scheduling, acceptance, packet curation (Dispatch Stubs), task-board (`SYSTEM_BLUEPRINT.md`), lock-stale decisions.
- **Architect**: Generates the Bi-Directional Traceability Matrix across the Macro/Meso/Micro hierarchy, defines Global Lock Topology, and establishes Static Boundaries for micro-features.
- **Designer**: Translates static boundaries into dynamic execution paths. Emits one meso-level spec and one meso-level test contract containing both unit and integration obligations.
- **Creator**: Translates the Designer's blueprints into Rust implementations one Creator Pass at a time, as sliced by the main agent.
- **Checker**: Validates behavior in synchronized Creator/Checker passes, owns independent meso-level integration passes, evaluates `qemu-serial.log` for runtime panics, and directly reports actionable repair instructions. *Holds the strict lock-guarded execution lane.*
- **Reviewer**: Performs post-checker static review on stabilized implementation passes, enforcing both line-level code quality and structural helper / owner-placement quality. Reviewer may directly edit only minor non-functional line-level issues; structural cleanup is rejected back to Creator. Reviewer does not own runtime verification.

## 3. Workflow Gates

The normal component path is:

```text
Planned -> Architected -> Specified
  -> One or more creator/checker pass loops
  -> Independent meso integration checker pass(es)
  -> Reviewer
  -> Optional final checker
  -> Accepted
```

Gate rules:
1. `Architected` means the Traceability Matrix and Static Lock Topology are established.
2. `Specified` means ALL Designer artifacts exist, explicitly covering dynamic lock behavior, unit-test obligations, and meso-level integration-test obligations.
3. The main agent must declare Creator Pass coverage before any implementation starts.
4. Creator-synced Checker passes must validate only the covered micro set of their matching Creator Pass.
5. Reviewer evaluates static code quality only after implementation and runtime validation stabilize; Reviewer is not a pre-checker gate for ordinary passes.
6. Reviewer acceptance requires both line-level `ASTERINAS_CODE_QUALITY_PRIORS.md` compliance and structural helper / owner-placement compliance.
7. Reviewer may approve without a final Checker only when the review edits are explicitly recorded as line-level and non-functional only; structural cleanup must not be hidden inside an "approved with edits" verdict.
8. `Accepted` requires full micro-feature coverage across passes, no blocking logic/quality findings, and verified exact-name execution for the required unit and meso integration tests.

## 4. Parallel Scheduling Model

Think in terms of one serialized command lane (Checker execution) plus as many safe command-free lanes as dependencies allow.

1. Keep the command lane narrow. Only actual execution logic requires the lock.
2. Keep others moving. While Checker holds the lock, Architect, Designer, Reviewer, and Creator lanes proceed if write-sets are disjoint.
3. The task assignment structure controls context size naturally.
4. Compile-only Creator exceptions are rare and consume the shared command environment.

### Conceptual Best-Effort Wave Example

Target workflow (concurrently active):

```text
command lane:
  A checker execution (holding lock)

command-free lanes:
  A checker pre-execution test scripting
  B creator pass (disjoint transaction)
  C designer (writing lock interaction contracts)
  D architect (mapping heavy specs to topology)
```
The workflow has one serialized execution lane but enables massive multi-phase concurrency.
