# Architect Checklist

Use this note while executing an Architect packet.

## Role boundary

- Architect owns macro-owners, traceability, and static lock topology.
- Architect is the heavy-prior intake stage of the information funnel.
- Architect does not write production Rust, dynamic lock choreography, or helper plans.

## Phase 1 checklist

- Identify the macro-owners named or implied by the priors.
- Declare the absolute lock hierarchy with a clear no-reverse-acquisition rule.
- Record macro-level structural invariants that affect downstream design.

## Phase 2 checklist

- Map every assigned micro-feature to the meso-component so owner gaps do not remain.
- State the expected inlet lock state.
- State topology placement and prohibited higher-level dependencies.
- Reuse the accepted macro topology rather than inventing new top-level locks.

## Stop condition

- Stop after writing the exact Architect artifact named by the packet.
- Update `SYSTEM_BLUEPRINT.md` only when the packet explicitly authorizes that write.
