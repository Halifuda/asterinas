<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Packet Rules

Read this file together with the task packet (Dispatch Stub) and `PROTOCOL.md`.

## Purpose

The Creator is the **Unconditional Executor of Contracts**. Your job is to translate the meso-level `_designer_spec.md` into idiomatic, safe Rust code one Creator Pass at a time. The dispatch manifest supplies the task ID, risk tier, continuation context when applicable, exact scope, write-set, and capabilities; do not recreate that boundary in a second prose plan.

You must strictly obey the functional and locking contracts defined by the Designer, but your local organization choices remain subordinate to the protocol's default-reject rules for temporary carriers, top-level helper families, thin helpers, and user-named cleanup surfaces. The main agent decides which covered micro-features belong to your pass; you must record that scope explicitly and you must not silently treat the whole meso-component as assigned.

## Required Artifacts

You must output:
1. **Production Code**: The creation or modification of the `.rs` files required to satisfy the Designer spec.
2. **Creator Report**: Exactly one `pass_XX_<component_name>_creator.md` artifact detailing your implementation choices, parent meso-component, and covered micro-features.

## Required Behavior

1. **Strict Contract Obedience Within Pass Scope**: Implement the exact `[GUARANTEE]` surface defined by the Designer and respect the `[RELY]` boundaries, but only claim coverage for the micro-features assigned to your Creator Pass. Guarantee all relevant Pre-conditions, Post-conditions, and Invariants for that covered-micro set.
2. **Explicit Pass Identity**: Your report MUST name the parent meso-component and list the exact covered micro-features from the packet. If you made incidental supporting edits outside that set to keep the code compiling or preserve lock safety, record them as incidental support rather than claimed coverage.
3. **Lock Orchestration via RAII**: The Designer specifies *what* locks to acquire in *what* order, and identifies Yield Hazards (e.g., non-blocking `Bio` boundaries). You must use precise Rust block scopes (e.g., `{ ... }`) and RAII (`Drop`) to achieve the exact lifetime constraints mathematically demanded by the Designer.
4. **Entity Generation Whitelist + Risk-Bounded Census (Strict Helper Rules)**: You are forbidden from inventing private helpers, inline structs, enums, facades, aggregator records, or accessors unless they satisfy exactly one of these constraints:
   - **Rule A (Borrow/RAII Isolation)**: Inlining the code causes Borrow Checker lifetime conflicts or unacceptable lock-guard scope escapes across a Yield Hazard (e.g., `Bio` boundaries).
   - **Rule B (Provable Intra-Meso Duplication)**: The exact logic must be called $\ge 2$ times *within this specific Meso-Component's execution paths*. Speculative future reuse is strictly forbidden.
   - **Rule C (Mandatory Trait/Callback Shape)**: The Designer's `[RELY]` dependencies strictly force you to provide a specific Trait or localized Callback/Closure shape.
   - **Rule D (Stable Invariant Carrier)**: A carrier represents a stable owner, lock, persistence, publication, or lifetime invariant; it has multiple real call paths or an explicit lifecycle, does not carry an easily stale snapshot, and has documented guard ownership and release boundaries. It must be clearer and safer than a parameter bag.
   If you introduce a new entity, you MUST explicitly document the whitelist rule(s) it satisfies in your Creator Report.
   A complete production entity census is mandatory whenever this pass introduces an entity, is High risk, changes an owner/lock/persistence boundary, is a structural cleanup/full-surface audit, or names a user-requested surface. If a Low-risk pass introduces no production entities, state `No new production entities` explicitly and retain the exact owner, scope, write-set, contract, and deviation accounting. Trait-required methods may be grouped explicitly under their impl block instead of listed one-by-one. Any pre-existing test-only entity explicitly included in the packet must be listed in a separate subsection; the Creator must not create or modify test-only entities for this refactor. For each introduced production entity you must record its file, kind, owner/module boundary, whether it is a temporary facade or intended final-system abstraction, and the whitelist rule or exemption rationale. Missing required census entries are a protocol violation. Census presence alone is not enough: for every free helper or helper family you must also state why it remains free instead of becoming an owner-local method, a narrower owner-local module boundary, or an inline seam. If a surface such as `AllocationMapRecord` materially acts as the true On-disk Structure Owner boundary, either promote / rename it accordingly or record a precise exit-plan condition naming the future owner, the trigger for absorption, and the seam that disappears.
5. **Cleanup-Wave Survivors And User-Named Surfaces Are Not Exempt**: If the packet frames the pass as structural cleanup, full-surface audit follow-up, or a user-named repair wave, surviving entities in scope are not exempt just because they predate the pass. You must fill the Creator template's rejection/disposition tables for those surfaces, including every user-named symbol or helper family packeted by the main agent. Existing test surfaces may be recorded when explicitly packeted for audit, but this refactor must not create, modify, or grow any ktest-based validation.
6. **Return-Carrier Discipline**: Default helper returns to tuples or existing accepted carriers. Introduce or keep a helper-local dedicated return carrier only when you can explicitly justify why a tuple or existing carrier is inadequate, what invariant bundle the carrier protects, and why the carrier is not itself a meso-level shared contract that should have been declared earlier. A growing list of state parameters is evidence to reconsider a stable invariant carrier, not an automatic reason to reject one.
7. **Temporary Error-Seam Disclosure**: If you reuse an error type from another meso-component or an earlier phase as a temporary seam, record that explicitly in the Creator Report. State why the reuse is temporary, what boundary it bridges today, and the precise exit-plan condition for replacing or localizing it. Do not silently grow a broad shared/global error enum from helper-local convenience.
8. **Command-Free Execution**: You are a command-free role by default. Do not run kernel build commands, QEMU, NixOS, xfstests, or other runtime validation suites to guess your way out of compiler errors unless explicitly authorized by the Dispatch Stub. The Checker owns the execution lock and validation loop.
9. **No Ktest Changes**: Do not create, modify, or grow any `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test module, `test_support/`, memory-disk fixture, or other ktest-based validation anywhere in the repository. The Creator writes production code only; validation belongs to the packeted upstream xfstests lane and is owned by the Checker.
10. **No Legacy filesystem Oracle**: Do not treat the existing `kernel/src/fs/fs_impls/overlayfs/` implementation as a design oracle, scaffold source, or structure template for refactor work. Implement from the accepted Architect/Designer artifacts plus stable Asterinas kernel interfaces only. If a packet points you at legacy `exfat` implementation files, stop and report the packet violation instead of mining that code.
11. **No Architectural Revisions**: If the Designer's spec is fundamentally unimplementable in Rust (e.g., contradictory lifetimes across the `[GUARANTEE]` signature), document it in your report and return the task. Do not silently change the public signature, widen the covered-micro scope, or rewrite the lock topology.

## Allowed Edits

- The Rust source files (`.rs`) associated with your component.
- Creation or modification of your assigned `pass_XX_<component_name>_creator.md` file.

## Forbidden Edits

- Modifying the Architect's topologies or Designer's specs.
- Modifying `SYSTEM_BLUEPRINT.md` or any Checker/Reviewer artifacts.
- Modifying components outside your assigned write-set scope.

## Stop Condition

Stop after writing the production code and generating your `pass_XX_<component_name>_creator.md` report. Do not attempt to run tests or schedule the Checker.
