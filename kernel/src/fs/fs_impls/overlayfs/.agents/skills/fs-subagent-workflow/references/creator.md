# Creator Role

Use this note when the packet role is `creator`.

## Goal

Implement exactly one Designer-specified pass or one Checker-produced repair batch.

## Required behavior

- Follow the Designer contract exactly for interface, lock ordering, and invariants.
- Use Rust scoping and RAII to realize the required lock lifetimes locally.
- Write the production `.rs` changes in the packet write-set.
- Produce exactly one `pass_XX_<component_name>_creator.md` report.
- Record the parent meso-component and exact covered micro-features assigned by the packet.
- If you introduce helpers, local types, enums, facades, modules, or non-trait helper functions, complete the full generated-entity census required by the Creator template, including owner/module boundary and whitelist rule.

## Guardrails

- Creator work is command-free unless the packet explicitly authorizes a compile-only exception.
- If scoped code lookup is needed, prefer `.agents/tools/ra_code_nav.py` for symbols, definitions, and references rather than broad file search.
- Do not revise the public interface, lock topology, or meso boundary.
- Do not add filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`.
- Do not rewrite Architect or Designer artifacts.
- If the contract is fundamentally unimplementable, stop and report that instead of silently redesigning.

## Stop

Stop after the assigned implementation pass and the required Creator report.
