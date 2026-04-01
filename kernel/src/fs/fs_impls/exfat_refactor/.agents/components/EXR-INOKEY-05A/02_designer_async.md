<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Keep this component synchronous and ownership-light.

The component is only about deterministic key derivation and exact opened-inode lookup. It does not introduce async work, background work, or filesystem-wide concurrency policy.

## Concurrency And Async Scope

- Shared state:
  - None introduced by the key helper itself.
  - Any opened-inode table state is read-only from this component's point of view.
  - The lookup payload type is generic and owned by later code.
- Locks:
  - None introduced by the key helper.
  - No lock ordering is needed for the canonical helper surface.
- Async operations:
  - None.
- I/O waiting:
  - None.
- Mutation policy:
  - The key helper is pure.
  - The lookup helper is read-only.
  - Registry mutation, if any exists later, is explicitly out of scope here.

## Required Behavior

- Keep the key derivation helper synchronous and deterministic.
- Keep the root constructor synchronous and explicit.
- Keep the opened-inode lookup helper read-only and exact-match only.
- Do not add futures, tasks, channels, atomics, mutexes, or condition variables.
- Do not make lookup depend on mount sequencing, inode lifecycle state, or directory traversal.
- Do not turn `fs.rs` into a filesystem owner or registry manager.

## Ownership Boundaries

- The future filesystem-wide state object remains owned by `EXR-MOUNT-09`.
- This component may reference shared opened-inode state only as a generic lookup table wrapper or test stub.
- If later work needs insertion, eviction, or parent-child propagation, that is a different component.
- If later work needs mount sequencing, that is a different component.

## Implications For Creator And Checker

- Creator work should stay confined to a pure key type and a generic read-only table wrapper.
- Checker work should remain synchronous and should not need async-specific harnesses.
- Any locking or synchronization needed by the eventual registry owner must be introduced elsewhere, not here.

## Non-Goals

- No async read path.
- No async write path.
- No background maintenance.
- No lock-order design.
- No shared mutable cache.
- No registry mutation policy.
- No mount lifecycle behavior.

## Exit Condition

The component is correctly designed when the reader can see that the entire pass is a synchronous identity helper plus exact lookup wrapper and nothing more.
