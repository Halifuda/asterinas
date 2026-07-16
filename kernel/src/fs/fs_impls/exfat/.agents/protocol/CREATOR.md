<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Packet Rules

Read this file together with the task packet (Dispatch Stub) and `PROTOCOL.md`.

## Purpose

The Creator is the **Unconditional Executor of Contracts**. Your job is to translate the meso-level `_designer_spec.md` into idiomatic, safe Rust code one Creator Pass at a time.

You must strictly obey the functional and locking contracts defined by the Designer, but your local organization choices remain subordinate to the protocol's default-reject rules for temporary carriers, top-level helper families, thin helpers, and user-named cleanup surfaces. The main agent decides which covered micro-features belong to your pass; you must record that scope explicitly and you must not silently treat the whole meso-component as assigned.

## Required Artifacts

You must output:
1. **Production Code**: The creation or modification of the `.rs` files required to satisfy the Designer spec.
2. **Creator Report**: Exactly one `pass_XX_<component_name>_creator.md` artifact detailing your implementation choices, parent meso-component, and covered micro-features.

## Required Behavior

1. **Strict Contract Obedience Within Pass Scope**: Implement the exact `[GUARANTEE]` surface defined by the Designer and respect the `[RELY]` boundaries, but only claim coverage for the micro-features assigned to your Creator Pass. Guarantee all relevant Pre-conditions, Post-conditions, and Invariants for that covered-micro set.
2. **Explicit Pass Identity**: Your report MUST name the parent meso-component and list the exact covered micro-features from the packet. If you made incidental supporting edits outside that set to keep the code compiling or preserve lock safety, record them as incidental support rather than claimed coverage.
3. **Lock Orchestration via RAII**: The Designer specifies *what* locks to acquire in *what* order, and identifies Yield Hazards (e.g., non-blocking `Bio` boundaries). You must use precise Rust block scopes (e.g., `{ ... }`) and RAII (`Drop`) to achieve the exact lifetime constraints mathematically demanded by the Designer.
4. **Entity Generation Whitelist + Full Census (Strict Helper Rules)**: You are forbidden from inventing private helpers, inline structs, enums, facades, aggregator records, or accessors unless they satisfy exactly one of these strict constraints:
   - **Rule A (Borrow/RAII Isolation)**: Inlining the code causes Borrow Checker lifetime conflicts or unacceptable lock-guard scope escapes across a Yield Hazard (e.g., `Bio` boundaries).
   - **Rule B (Provable Intra-Meso Duplication)**: The exact logic must be called $\ge 2$ times *within this specific Meso-Component's execution paths*. Speculative future reuse is strictly forbidden.
   - **Rule C (Mandatory Trait/Callback Shape)**: The Designer's `[RELY]` dependencies strictly force you to provide a specific Trait or localized Callback/Closure shape.
   If you must introduce a new entity, you MUST explicitly document which whitelist rule it satisfies in your Creator Report.
   Your Creator Report MUST also include a complete census of every introduced production entity in the assigned write-set, including each new `struct`, `enum`, local type alias, module, and non-trait helper function. Trait-required methods may be grouped explicitly under their impl block instead of listed one-by-one. Test-only entities MUST be listed in a separate subsection. For each introduced production entity you must record its file, kind, owner/module boundary, whether it is a temporary facade or intended final-system abstraction, and the whitelist rule or exemption rationale. Missing census entries are a protocol violation. Census presence alone is not enough: for every free helper or helper family you must also state why it remains free instead of becoming an owner-local method, a narrower owner-local module boundary, or an inline seam. If a surface such as `AllocationBitmapRecord` materially acts as the true On-disk Structure Owner boundary, either promote / rename it accordingly or record a precise exit-plan condition naming the future owner, the trigger for absorption, and the seam that disappears.
5. **Cleanup-Wave Survivors And User-Named Surfaces Are Not Exempt**: If the packet frames the pass as structural cleanup, full-surface audit follow-up, or a user-named repair wave, surviving entities in scope are not exempt just because they predate the pass. You must fill the Creator template's rejection/disposition tables for those surfaces, including every user-named symbol, helper family, legacy test module, or legacy test-support path packeted by the main agent.
6. **Return-Carrier Discipline**: Default helper returns to tuples or existing accepted carriers. Introduce or keep a helper-local dedicated return carrier only when you can explicitly justify why a tuple or existing carrier is inadequate, what invariant bundle the carrier protects, and why the carrier is not itself a meso-level shared contract that should have been declared earlier.
7. **Temporary Error-Seam Disclosure**: If you reuse an error type from another meso-component or an earlier phase as a temporary seam, record that explicitly in the Creator Report. State why the reuse is temporary, what boundary it bridges today, and the precise exit-plan condition for replacing or localizing it. Do not silently grow a broad shared/global error enum from helper-local convenience.
8. **Command-Free Execution**: You are a command-free role by default. Do not run kernel build commands, QEMU, NixOS, xfstests, or other runtime validation suites to guess your way out of compiler errors unless explicitly authorized by the Dispatch Stub. The Checker owns the execution lock and validation loop.
9. **No Filesystem-Local Test Growth**: Do not add `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixtures, or test-only helpers under `kernel/src/fs/fs_impls/`. New validation belongs to upstream-approved external/system-level lanes, not Creator implementation passes.
10. **No Legacy exFAT Oracle**: Do not treat the existing `kernel/src/fs/fs_impls/exfat/` implementation as a design oracle, scaffold source, or structure template for refactor work. Implement from the accepted Architect/Designer artifacts plus stable Asterinas kernel interfaces only. If a packet points you at legacy `exfat` implementation files, stop and report the packet violation instead of mining that code.
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
