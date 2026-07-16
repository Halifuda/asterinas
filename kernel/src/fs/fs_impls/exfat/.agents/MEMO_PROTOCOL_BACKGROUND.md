<!-- SPDX-License-Identifier: MPL-2.0 -->

# Background & Genesis of the Top-Down Strict Protocol

**Date:** April 14, 2026
**Context:** This memo records the rationale and historical context behind the deprecation of the legacy `.agents/` workflow and the establishment of the `new_protocol/` architecture for the `exfat_refactor` project.

## The 0413 Concurrency Crisis
The legacy protocol (the "fast26-liu" serial-first, async-supplement model) collapsed during the implementation of complex filesystem operations, most notably during the `read_at` and cache refilling flows. This failure affectionately became known as the "0413 Concurrency Crisis."
The core issue was that the old protocol treated concurrency and lock management as an *afterthought*—a bolt-on detail to be patched in later. LLM agents, lacking a human's "subconscious global context," naturally struggled to infer safe lock boundaries and hierarchies on the fly, leading to:
- Self-deadlocks caused by unmanaged lock scopes around block I/O.
- Inappropriate injection of `async`/`await` into the synchronous kernel environment.
- Missing feature implementations (like Volume Label omissions) because they fell through the cracks of loosely defined task requirements.

## The Paradigm Shift: Top-Down Restraints
To resolve this, the LLM multi-agent workflow was fundamentally overhauled. Concurrency and hierarchy can no longer emerge organically; they must be foundational and rigorously defined before a single line of Rust is written.

### 1. The Information Funnel
The legacy protocol suffered from severe token bloat and "over-prompting." Main-agents would dump entire exFAT specifications and scheduling minutiae onto downstream Creators. This context overflow encouraged simple coding agents to "play Architect" and invent unauthorized structures or blocking logic.
The new protocol enforces a **Strict Information Funnel**:
- **Architects** absorb heavy documentation (specs, Linux sources) to map the system.
- **Designers** absorb the Architect's topology to forge dynamic lock contracts.
- **Creators** receive *only* the Designer's contract and basic code quality rules. They are command-free, unconditional executors.

### 2. Role Overhaul & Component Hierarchy
- **Architect:** Shifts from a module slicer to a system defense builder. They define the Global Lock Topology and trace features to macro-owners.
- **Designer:** Solves "Dynamic Lock Problems." They enforce strict Lock Interaction Contracts and non-blocking path boundaries (`Bio` block I/O).
- **Creator:** Transformed into a mindless construction arm focused strictly on Rust semantics (`Drop`, `?`) within Designer-mandated bounds.
- **Checker:** Absorbed the old Advisor role. They now own the execution lock, evaluate QEMU serial logs directly, and produce actionable repair batches.

### 3. Pipeline Rigidity
- **Template Acceptance:** The Main-Agent now evaluates Subagents structurally, not logically. If a required section of the template (e.g., Rely-Guarantee proofs) is empty, the artifact is rejected.
- **5-Retry Escalation Path:** To prevent infinite code/test loops, if a Creator/Checker cycle fails 5 times, it is forcefully escalated to the Designer/Architect level to rethink the architecture.

This `new_protocol` is designed to be a cold, mechanical, state-driven assembly line, stripping away the LLM's tendency to hallucinate architecture and forcing it into a provably safe concurrency model for the Asterinas kernel.
