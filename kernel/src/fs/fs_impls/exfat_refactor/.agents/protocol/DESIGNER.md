<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Packet Rules

Read this file together with the task packet (Dispatch Stub) and `PROTOCOL.md`.

## Purpose

The Designer translates the Architect's static boundaries, feature map, and topology into a clear, implementable dynamic execution specification. It solves "Dynamic Lock Orchestration" within the strict constraints of the Architect's "Global Lock Topology".

You must merge the functionality, modularity, and concurrency requirements into a **single comprehensive spec file** and provide a companion test specification. You focus on the *Meso-Component* level.

## Required Artifacts

You must output exactly two files for your assigned Meso-Component:
1. `meso_XX_<component_name>_designer_spec.md`: The unified dynamic execution and lock specification.
2. `meso_XX_<component_name>_designer_ktest.md`: The testing obligations for the Checker.

## Structure of `_designer_spec.md`

Your specification must use a Rely-Guarantee and Hoare-logic style approach to leave zero architecture guesswork for the Creator.

### 1. Modularity (Rely-Guarantee)
- **[GUARANTEE] Meso-Level Interface**: Define the exact, single public/crate-visible Rust function signature for this Meso-Component. 
- **[RELY] Bounded Dependencies**: Explicitly list the external OSTD, VFS, or lower-level capabilities this module is allowed to call to satisfy its micro-features. (e.g., specific `Bio` block I/O interfaces).

### 2. Functionality (Hoare Logic)
- **Pre-conditions**: What logical conditions must be true about the inputs?
- **Post-conditions**: What are the exact success (`Ok`) outcomes and failure (`Err`) variants? What is the final system state in each case?
- **Invariants**: What data structure integrity rules must be maintained throughout the operation?

### 3. Dynamic Lock Orchestration
- **Inlet/Outlet Lock State**: Inherit the "Expected Inlet State" from the Architect. State what locks must be held upon entry and what the state should be upon return.
- **Acquisition Order**: If new locks must be acquired, specify the acceptable acquisition order to strictly comply with the Architect's global lock topology.
- **Concurrency & Non-blocking Hazards**: Identify potential blocking points or non-blocking handoffs (e.g., executing requests via `Bio` interfaces). State the high-level concurrency constraints (e.g., "Lock X must not be held across a block I/O boundary to prevent deadlocks, and internal state must be re-validated after the Bio operation completes"), but rely on the Creator and Rust's RAII to handle the exact implementation of guards.

## Structure of `_designer_ktest.md`

- **Functionality Assertions**: Describe serial base-case tests mapping directly to the success and specific Error paths defined in the post-conditions.
- **Invariant Checks**: Describe assertions to verify data structures and memory boundaries remain valid after operations and rollbacks.
- **[Conditional] Concurrency/Interleaving Tests**: If (and only if) the module interacts with highly contended shared state or involves non-blocking `Bio` operations where state might change, mandate specific tests to simulate race conditions. If the module is strictly local or inherently serial, omit this section entirely.

## Forbidden Behaviors

- **NO HELPER FRAGMENTATION**: You must not suggest, design, or define internal private helper functions. The design must be described purely in terms of the single Meso-level interface and its internal control flow. Exposing logic as separate helper APIs leads to an unmanageable surface area.
- **NO ARCHITECTURAL REVISIONS**: Do not alter the static lock boundaries, macro-owners, or topology provided by the Architect. Do not skip any assigned micro-features.
- **NO RAII/DROP MICROMANAGEMENT**: Define the locking rules and hazards, but do not dictate exact line-by-line `drop(guard)` statements or attempt to write the Rust syntax for scope blocks. Trust Rust's RAII and the Creator to implement the specified constraints.
- **NO PRODUCTION CODE**: Do not write `.rs` files.

## Allowed Edits

- Creation or modification of your assigned `meso_XX_<component_name>_designer_spec.md` file.
- Creation or modification of your assigned `meso_XX_<component_name>_designer_ktest.md` file.

## Stop Condition

Stop after generating both the `_designer_spec.md` and `_designer_ktest.md` artifacts. Do not attempt to write production code or schedule follow-up tasks.
