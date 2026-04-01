<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: EXR-INODE-05B
- Title: Read-Only Inode Metadata Shell
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Keep this inode shell synchronous, immutable after construction, and free of ownership policy.

The component is a pure metadata value object. It does not introduce async work, background work, cache maintenance, mount sequencing, or registry mutation.

## Concurrency And Async Scope

- Shared state:
  - None introduced by the shell itself.
  - Any sharing after construction is by immutable reference or by moving a read-only value object around.
- Locks:
  - None introduced by the shell itself.
  - No lock ordering is needed for the constructor or accessors.
- Async operations:
  - None.
- I/O waiting:
  - None.
- Mutation policy:
  - Construction is one-time value assembly.
  - Accessors are read-only.
  - No helper should depend on a mutable filesystem owner, an inode registry, or page-cache ownership.

## Required Behavior

- Keep the ordinary constructor synchronous and deterministic.
- Keep the root constructor synchronous and explicit.
- Keep all accessors pure.
- Do not add futures, tasks, channels, mutexes, atomics, or condition variables.
- Do not make the shell depend on mount sequencing, directory traversal, or lookup mutation.
- Do not add `PageCache`, `PageCacheBackend`, or any buffering policy.

## Ownership Boundaries

- `EXR-PGCACHE-11B` remains the owner of page-cache backend integration.
- `EXR-MOUNT-09` remains the owner of filesystem-wide mount sequencing and shared runtime state.
- `EXR-DIR-10` remains the owner of directory iteration and lookup policy.
- This component may be read by those later slices, but it must not absorb their responsibilities now.

## Implications For Creator And Checker

- Creator work should stay confined to a synchronous metadata shell in `inode.rs`.
- Checker work should remain synchronous and should not need any async harness.
- Any state that would require coordination across threads or background tasks belongs in a later component.

## Non-Goals

- No async read path.
- No async write path.
- No deferred flush or background invalidation.
- No lock-order design.
- No shared mutable cache.
- No registry mutation policy.
- No mount lifecycle behavior.

## Exit Condition

The component is correctly designed when a reader can see that the entire pass is a synchronous read-only metadata shell and nothing more.
