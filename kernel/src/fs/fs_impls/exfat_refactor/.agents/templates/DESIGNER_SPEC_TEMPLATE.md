<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification Template

## Metadata

- Component ID:
- Title:
- Status: `Specified`
- Author:
- Date:
- Based on architect artifact:

## Scope

- In scope:
- Out of scope:

## Module Specification

- Dependencies:
- Interfaces provided:
- Files/modules touched:
- Hidden implementation details:

## Functional Specification

For each externally visible operation, write a precondition/action/postcondition rule.

### Operation

- Name:
- Inputs:
- Preconditions:
- Actions:
- Outputs:
- Postconditions:
- Error cases:

Repeat the operation block as needed.

## Invariants

List the invariants this component establishes or preserves.

## Concurrency Specification

- Shared state:
- Locking or serialization assumptions:
- Required atomicity:
- Forbidden interleavings:
- Behavior under concurrent readers/writers:

## Tests and Observability

- Checker-owned unit or kernel tests expected:
- Observable behaviors the checker should verify:
Note any test comments that are important for readability when the scenario is not obvious from the test body alone.

## Creator Notes

State any constraints the creator must not silently reinterpret.
Do not assign test-writing work to the creator unless the main agent has explicitly overridden the default checker-owned test policy.
