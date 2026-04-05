<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `SerialChecked`
- Author: `main-agent`
- Date: `2026-04-05`
- Task packet: locally executed by `main-agent` in the `ember-causeway` wave; no delegated checker packet
- Checked implementation:
  - `10_creator_serial.md`
- Pass kind: `serial`

## Scope of Review

Checked the new `fs.rs` mount/bootstrap implementation plus its narrowly supporting helper changes in `fat.rs`, `inode.rs`, `bitmap.rs`, and `mod.rs` against the accepted `EXR-MOUNT-09` architect and designer artifacts.

## Test Changes

Added local `#[ktest]` coverage in `fs.rs` for:

- successful bootstrap publication from prevalidated superblock and root facts,
- rejection of missing bitmap or upcase discovery facts,
- explicit synthetic-root seeding through `ExfatInodeMeta::new_root(...)`,
- bootstrap failure atomicity when a dependent loader rejects the discovered upcase payload.

The tests stay local to `fs.rs` and do not require lookup orchestration, page cache, mutation helpers, or an async harness.

## Findings

No remaining blocking findings.

The first checker execution found one local test-harness defect:

- `unwrap_err()` required `ExfatFs: Debug`, which would have widened the production mount surface for no boundary reason.

That defect was repaired locally by rewriting the tests to destructure `Result` explicitly, and the focused retry then passed cleanly.

## Verified Properties

- `ExfatFs::mount(...)` consumes accepted root facts rather than rediscovering root state.
- The happy path publishes one complete filesystem object containing the block device, validated superblock copy, loaded upcase table, loaded allocation bitmap, and synthetic root seed.
- Missing bitmap or upcase discovery facts are rejected at the mount boundary.
- The root seed matches the explicit synthetic-root constructor path.
- A dependent-loader failure returns an error instead of publishing partial state.
- Focused exact-name `cargo osdk test` runs passed under `.agents/tools/checker_lock.sh` in the TCG-backed container environment.
- The recorded filters are the exact local ktest function names:
  - `mount_happy_path_publishes_complete_shared_state`
  - `mount_rejects_missing_root_discovery_facts`
  - `mount_root_seed_uses_synthetic_root_constructor`
  - `mount_failure_is_atomic_when_loader_rejects_dependency`
- Those filters were treated as valid coverage because `cargo osdk test` matches test-path suffixes, so each exact function name maps to the intended `fs.rs` ktest by source inspection.

## Unverified Properties

- No reviewer pass has run yet, so post-review stability is still unverified.
- The broader downstream consumer shape for `ExfatFs` is intentionally deferred to later components.

## Recommendation

- Next owner: `main-agent`
- Reason: run the bounded reviewer pass and then the normal post-review final checker.
- Blocking or non-blocking: non-blocking
