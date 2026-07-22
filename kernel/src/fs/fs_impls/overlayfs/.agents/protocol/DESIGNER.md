<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Packet Rules

Read this file together with the task packet (Dispatch Stub) and `PROTOCOL.md`.

## Purpose

The Designer translates the Architect's static boundaries, feature map, and topology into a clear, implementable dynamic execution specification. It solves "Dynamic Lock Orchestration" within the strict constraints of the Architect's "Global Lock Topology".

You must merge the functionality, modularity, and concurrency requirements into a **single comprehensive spec file** and provide a minimal companion external-evidence specification. You focus on the *Meso-Component* level. The main agent will later slice your meso-level contract into Creator Passes, so your artifacts must stay meso-scoped and explicitly traceable back to named micro-features. Your job is to define the semantic boundary and behavioral obligations of the meso, not to freeze concrete Rust signature spelling, carrier family names, dispatcher type names, or validation-harness mechanisms. A Designer task may be an initial contract or a bounded revision continuation; a revision may substantially rewrite both artifacts while preserving the parent Meso, covered Micro set, and accepted Architect topology. If the static owner or lock topology is wrong, report it for Architect repair instead of changing it silently. The evidence artifact maps those obligations to upstream xfstests; it is not a plan for internal tests. This refactor uses xfstests as its sole validation lane and must not create, modify, or imply any ktest or other internal test surface.

## Required Artifacts

You must output exactly two files for your assigned Meso-Component:
1. `meso_XX_<component_name>_designer_spec.md`: The unified dynamic execution and lock specification.
2. `meso_XX_<component_name>_designer_validation.md`: The upstream-approved external-validation mapping and integration obligations for the Checker.

## Structure of `_designer_spec.md`

Your specification must use a Rely-Guarantee and Hoare-logic style approach to leave zero architecture guesswork for the Creator.
When a branch, invariant, or hazard only applies to specific micro-features, name those micro-features explicitly so later pass slicing stays deterministic.

### 1. Modularity (Rely-Guarantee)
- **[GUARANTEE] Meso-Level Boundary**: Define the single semantic crate-visible boundary for this Meso-Component: what class of request enters, what class of result leaves, and what must remain internal control flow beneath that boundary. Do **not** prescribe an exact Rust function signature, exact type names, enum names, or variant spelling unless the packet explicitly says you are documenting an already-fixed pre-existing kernel interface.
- **[RELY] Bounded Dependencies**: Explicitly list the external OSTD, VFS, or lower-level capabilities this module is allowed to call to satisfy its micro-features. (e.g., specific `Bio` block I/O interfaces).

### 2. Functionality (Hoare Logic)
- **Pre-conditions**: What logical conditions must be true about the inputs?
- **Post-conditions**: What classes of success and failure must exist, and what is the final system state in each case? You may name semantic cases, but do not invent or freeze exact Rust enum variant spelling unless the packet explicitly authorizes documenting a pre-existing stable interface.
- **Invariants**: What data structure integrity rules must be maintained throughout the operation?

### 3. Dynamic Lock Orchestration
- **Inlet/Outlet Lock State**: Inherit the "Expected Inlet State" from the Architect. State what locks must be held upon entry and what the state should be upon return.
- **Acquisition Order**: If new locks must be acquired, specify the acceptable acquisition order to strictly comply with the Architect's global lock topology.
- **Concurrency & Non-blocking Hazards**: Identify potential blocking points or non-blocking handoffs (e.g., executing requests via `Bio` interfaces). State the high-level concurrency constraints (e.g., "Lock X must not be held across a block I/O boundary to prevent deadlocks, and internal state must be re-validated after the Bio operation completes"), but rely on the Creator and Rust's RAII to handle the exact implementation of guards.

### 4. Representation and Complexity Guidance

*This section is advisory and must not freeze Rust type names or Creator Pass
boundaries. When the packet requests it, record the expected owner/carrier
shape, relevant lock or publication coordinators, and a small complexity
baseline for the Meso.*

- **Stable Invariant Carriers Allowed:** [State the owner, lock, persistence,
  or lifetime invariant that justifies a carrier and its guard boundary.]
- **Carrier / Helper Boundaries:** [State which temporary or thin helpers are
  rejected and which stable protocol objects may remain.]
- **Complexity Baseline / Budget:** [Record advisory counts for new entities,
  long-parameter functions, temporary carriers, coordination objects, or
  repeated specification text. Explain deliberate budget overruns.]
- **Revision Disposition:** [For a revision continuation, list changed
  obligations, preserved obligations, and any Architect escalation.]

## Structure of `_designer_validation.md`

### 1. External Validation Mapping

Map every assigned micro-feature to the upstream-approved xfstests case or
group that is expected to exercise it. The mapping is many-to-many: one test
may cover several micro-features, and one micro-feature may require several
tests. For each row, record the expected observation and classify the evidence
as `direct`, `combined`, `not-run/unsupported`, or `no upstream coverage` when
the suite cannot isolate or exercise the feature. Record an upstream coverage
gap as a limitation; do not invent another validation lane to compensate for
it.

### 2. Pass-Scoped Checker Obligations

State which mapped xfstests are relevant to a Creator-synced Checker for its
exact Creator Pass scope. The Checker must preserve the scope boundary even
when a selected test exercises neighboring micro-features. Require reporting
of the actual test IDs, result files, guest logs, and any `PASS`, `FAIL`, or
`NOTRUN` classification; do not require or imply a separate test per
micro-feature.

### 3. Invariant and Integration Observations

Describe externally observable invariant, rollback, remount, persistence, or
deadlock observations that xfstests can provide. Treat logs and suite results
as runtime evidence, not as a proof of memory safety or internal
implementation correctness. Describe tightly coupled
meso-level scenarios as separate integration Checker passes. Each scenario
must explain `Setup`, `Execution Chain`, and `Assertion`, but remain high-level
rather than line-by-line pseudocode. Include a success path whenever the meso
has non-trivial cross-feature interaction. Add failure-maintenance,
idempotence, or concurrency paths only when an upstream test exists; otherwise
state that the upstream lane provides no such scenario and leave the gap
explicit.

The current and sole validation lane is NixOS-driven xfstests unless the
upstream project standardizes a different external filesystem-validation
route. If an obligation has no upstream coverage, record that limitation;
do not add an internal test lane.

## Forbidden Behaviors

- **NO SIGNATURE DESIGN**: You must not invent or freeze exact Rust function signatures, exact carrier type names, dispatcher families, or enum variant spelling for a new meso boundary. Describe semantic inlet/outlet classes and behavioral obligations instead. If the packet explicitly points to a pre-existing stable kernel interface, you may cite that interface as inherited context, but you still must not design new signature shapes around it.
- **NO HELPER FRAGMENTATION**: You must not suggest, design, or define internal private helper functions. The design must be described purely in terms of the single meso boundary and its internal control flow. Exposing logic as separate helper APIs leads to an unmanageable surface area.
- **NO ARCHITECTURAL REVISIONS**: Do not alter the static lock boundaries, macro-owners, or topology provided by the Architect. Do not skip any assigned micro-features.
- **NO PASS SLICING**: Do not decide Creator Pass boundaries or say "Pass 1 should implement X and Y." That is owned by the main agent.
- **XFSTESTS-ONLY VALIDATION**: Do not request, design, create, modify, or imply any internal unit-test lane, `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test module, `test_support/`, memory-disk fixture, or other ktest harness anywhere in the repository. Validation must be expressible through the upstream xfstests lane. Any xfstests harness/configuration change must be outside `kernel/src/fs/fs_impls/` and explicitly authorized by the packet.
- **NO RAII/DROP MICROMANAGEMENT**: Define the locking rules and hazards, but do not dictate exact line-by-line `drop(guard)` statements or attempt to write the Rust syntax for scope blocks. Trust Rust's RAII and the Creator to implement the specified constraints.
- **NO PRODUCTION CODE**: Do not write `.rs` files.

## Allowed Edits

- Creation or modification of your assigned `meso_XX_<component_name>_designer_spec.md` file.
- Creation or modification of your assigned `meso_XX_<component_name>_designer_validation.md` file.

## Stop Condition

Stop after generating both the `_designer_spec.md` and `_designer_validation.md` artifacts. Do not attempt to write production code or schedule follow-up tasks.
