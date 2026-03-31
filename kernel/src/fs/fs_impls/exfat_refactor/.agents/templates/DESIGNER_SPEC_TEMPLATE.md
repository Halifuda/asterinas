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
- Files or modules touched:
- Hidden implementation details:

## Functional Specification

Describe the component's externally relevant behavior in precondition or action or postcondition form.

## Invariants

List the invariants that later passes may rely on.

## Concurrency Specification

- Shared state:
- Lock ordering:
- Atomicity requirements:
- Forbidden interleavings:
- Allowed simplifications such as a temporary big lock:

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
- Explicit non-goals:

### Serial Checker Pass

- Required checker-owned tests:
- Observable properties that must pass before leaving the serial loop:

### Concurrency Creator Pass

- Required implementation obligations:
- Explicit non-goals:

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
- Write `No dedicated concurrency tests required` if the concurrency spec does not need them.
- Observable properties that must pass before leaving the concurrency loop:

## Acceptance Notes

List anything the reviewer should pay special attention to later.
