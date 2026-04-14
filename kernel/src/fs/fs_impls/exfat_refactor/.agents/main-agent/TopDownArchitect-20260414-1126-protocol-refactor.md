<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff: Top-Down Contracts Refactor

Treat this file as the editable record of the current main-agent wave.

## Metadata

- Fancy nickname: TopDownArchitect
- Date: April 14, 2026
- Covered hours: Q2 2026 Shift
- Author: Main Agent
- Workspace: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Container or environment: Asterinas NixOS dev container
- Status: Active Planning

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260319`
- Working path: `/home/halifuda/asterinas`
- Container name, if any: `N/A`
- KVM status: Active
- Validated commands: Verified compilation & testing capabilities
- Known environment blockers: Single serialized execution lane (QEMU lock) remains.

## Current Project State

- Current goal: Refactor protocol to enforce strict Top-Down Multi-Agent workflow, mitigating self-deadlocks and specification omissions.
- Current phase: Protocol Redesign (Architect/Designer/Creator stages)
- Active or next component: Updating `.agents/protocol/` documents
- Latest accepted components: None assigned in this wave yet.
- Components in progress: Protocol updates
- Blocked components: N/A

## Active Work Slice Matrix

None.

## Recent Decisions

- Concurrency bugs (e.g., self-deadlocks caused by injecting `async` on top of sync `read_at`) and missing features (e.g., Volume Label omissions) stem from loose prior protocols.
- The protocol is shifting to a strict Top-Down model:
  1. **Architect**: Now responsible for establishing domain structures, global Lock Topology, and generating a rigorous Bi-Directional Traceability Matrix (mapping exFAT Spec & VFS Flags to sub-modules explicitly).
  2. **Designer**: Operates strictly as a "Contract Specifier," establishing definite Lock Interaction Contracts and Path Boundary Restraints (i.e., non-blocking mandates).
  3. **Creator**: High-token self-reviews are replaced with low-overhead, unconditional obedience to the Designer's Lock/Path Contracts, focusing closely on Rust's `Drop` semantics and early-return (`?`) synchronization invariants.

## Wave Record

- Identified systemic architectural weaknesses in current implementation outputs (as detailed in `/mnt/c/Users/anyud/Documents/storagelab/中关村实验室/0415.md`).
- Designed a 3-layer reform prioritizing kernel concurrency constraints and feature traceability.
- Scheduled protocol modifications across Architect, Designer, and Creator rule sets.
- Replaced the incorrectly formatted handoff with this templated variant to align with `README.md` conventions.

## Open Risks And Assumptions

- Subagent token constraints limit the size of generated mapping matrices; the Architect might need instruction parsing strategies when generating massive Bi-Directional Traceability maps.
- Strict enforcement of non-blocking paths requires adequate pre-existing lower-level primitives in Asterinas. If absent, the Architect/Designer will need to provision entirely isolated dual-path structures without resorting to fallback blocking interfaces.

## Recommended Next Actions

1. Update `.agents/protocol/ARCHITECT.md` to specify the production of macro-owner structures, Global Lock Topology generation, and Bi-Directional Traceability Matrices.
2. Overhaul `.agents/protocol/DESIGNER.md` to pivot from architectural design into establishing strict Lock Interaction Contracts and Path/Concurrency boundaries for every target sub-module.
3. Patch `.agents/protocol/CREATOR.md` to substitute broad functional reviews with specific rule-checks on kernel context compliance (e.g., lock lifecycle during early returns, respecting Designer's interface bounds).
