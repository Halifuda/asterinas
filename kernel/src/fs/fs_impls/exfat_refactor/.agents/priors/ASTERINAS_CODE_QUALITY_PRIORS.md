<!-- SPDX-License-Identifier: MPL-2.0 -->

# Asterinas Code Quality Priors For exFAT

This note records the code-quality rules that the main agent may slice into task packets.
It is derived from:

- the repository-root `AGENTS.md`,
- `book/src/to-contribute/coding-guidelines/README.md`,
- the guideline pages linked from that book index.

This file is not a semantic authority for exFAT behavior.
It governs engineering quality: component shape, API boundaries, naming, comments, safety, testing, and review expectations.

Packets should normally reference the role profiles or section ranges below instead of pasting long prose.
The point of this file is to reduce repeated manual rediscovery, not to force every role to read the whole coding-guidelines book on every pass.

## 1. How To Use This File

Use code-quality prior slices by role:

- architect gets boundary-level quality rules, not creator-level micro-implementation rules,
- designer gets contract-level quality rules, not a full implementation style manual,
- creator gets the implementation-quality slice relevant to the touched files,
- checker gets the verification-quality slice plus any implementation-quality rules that affect defect classification,
- reviewer gets the broadest quality slice because review is explicitly about code quality.

If a packet already cites the exact relevant quality sections, a role should not silently widen scope to the full file.

## 2. Packet-Friendly Role Profiles

### `Q-ARCH`

Use for architect packets.
Focus on:

- one concept per file or component,
- top-down readable decomposition,
- explicit trust and validation boundaries,
- ownership boundaries that keep later live behavior separate from early value layers,
- avoiding components that are line-budget compliant but still too broad in responsibilities.

Architect packets should not use this profile to pre-specify local implementation trivia that properly belongs to creator judgment.

### `Q-DESIGN`

Use for designer packets.
Focus on:

- naming the canonical interface surface instead of several overlapping helpers,
- recording invariants and failure cases explicitly,
- helper justification with named downstream callers or boundaries,
- temporary staging interfaces with future owner and removal condition,
- keeping creator-facing specs implementable without guesswork but without dictating unnecessary microstructure.

Designer packets should not use this profile to force creator-local naming or ordering choices unless correctness or boundary clarity depends on them.

### `Q-CREATE`

Use for creator packets.
Focus on:

- descriptive and accurate names,
- smallest practical visibility, usually `pub(super)` or `pub(crate)`,
- small focused functions and low nesting,
- checked or saturating arithmetic,
- error propagation with `?` instead of fallible `unwrap()`,
- types that encode invariants where practical,
- comments that explain why, not what,
- doc-comment style from the repository guidelines,
- avoiding speculative helpers, redundant wrappers, and field-exposing accessors without proof of need,
- citing specification or algorithm sources when implementing non-obvious behavior.

### `Q-CHECK`

Use for checker packets.
Focus on:

- validating user-visible or contract-visible behavior instead of only implementation internals,
- adding regression-oriented tests where behavior could drift,
- using assertion macros instead of manual inspection,
- keeping `#[ktest]` cases local, readable, and scenario-labeled,
- treating undocumented temporary interfaces, unjustified helpers, unsafe usage in `kernel/`, or obvious code-quality contract violations as real defects when they are inside scope.

### `Q-REVIEW`

Use for reviewer packets.
Focus on:

- checked arithmetic and boundary validation,
- naming accuracy,
- visibility hygiene,
- type-level invariant expression,
- top-down readability,
- comment and doc-comment quality,
- concurrency hygiene such as lock ordering and avoiding blocking under spinlocks when relevant,
- avoiding casual atomics,
- avoiding unnecessary copies, allocations, and hot-path O(n) logic when relevant,
- ensuring test helpers do not hide production mistakes.

## 3. Core Quality Rules Most Relevant To This Refactor

The following rules recur often enough in `exfat_refactor` that they deserve stable packet references:

1. Explain why, not what.
2. Document non-obvious design decisions and cite specifications when implementing them.
3. Keep one concept per file or component.
4. Organize code for top-down reading.
5. Hide implementation details and default to narrow visibility.
6. Validate at trusted boundaries; do not keep revalidating already-validated facts everywhere.
7. Use checked or saturating arithmetic.
8. Use types to encode invariants where practical.
9. Keep helpers purposeful; avoid tiny wrappers that only expose stored data without a proven caller need.
10. Mark temporary staging interfaces explicitly with owner and exit condition.
11. In `kernel/`, do not introduce `unsafe`.
12. Use `?` for fallible flows and avoid `unwrap()` where failure is possible.
13. Keep tests readable, assertion-based, and close to the behavior they validate.
14. Keep test-only helpers in `mod tests` or test-only support modules by default; `#[cfg(ktest)]` items inside production code need an explicit cross-module reason and an exit plan.

## 4. Role Boundaries For Quality Guidance

Quality guidance must not collapse role boundaries:

- architect should define better boundaries, not write creator-local implementation plans,
- designer should define contracts and invariants, not micromanage formatting or trivial local naming,
- creator should own implementation details inside the approved boundary,
- checker should convert missing coverage and quality-contract violations into concrete findings,
- reviewer should clean up bounded quality issues without redesigning the component.

If a packet uses code-quality guidance to push one role into another role's job, the packet is too broad and should be narrowed.

## 5. Packet-Efficiency Rules

To keep token cost under control:

- prefer profile labels such as `Q-CREATE` or `Q-REVIEW`,
- when possible, cite only the exact sections needed for the current component,
- do not paste the full contents of the root `AGENTS.md` or the coding-guidelines book into ordinary packets,
- if an architect or designer already distilled the relevant quality constraints faithfully, later packets may cite that artifact instead of restating the same quality prose.

The goal is stable reuse, not repeated expansion.
