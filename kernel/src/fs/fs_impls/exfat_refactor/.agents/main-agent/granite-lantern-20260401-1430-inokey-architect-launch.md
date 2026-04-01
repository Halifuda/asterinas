<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `granite-lantern`
- Date: 2026-04-01 14:30 CST
- Author: main-agent
- Covered hours: approximately `3.9` hours, from `2026-04-01 14:30 CST` to `2026-04-01 18:30 CST`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Context restored, environment revalidated, `EXR-INOKEY-05A` and `EXR-INODE-05B` accepted, protocol tightened again, and post-acceptance helper cleanup verified under TCG

## Environment Summary

- Host workspace: `/home/halifuda/asterinas`
- Container workspace: `/root/asterinas`
- Container name: `codex-asterinas-dev`
- KVM status: still `no-kvm` inside the current container
- Revalidated commands:
  - `docker ps --format '{{.Names}}'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && pwd && test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
- Continuity notes:
  - The shared container assumptions from `iron-ridge-20260401-1052-sbgeom-repair-and-inokey-prep.md` still hold.
  - This wave has not yet started any build, test, or QEMU-producing work.

## Current Project State

- Immediate planning target: choose the next architect wave after `EXR-INODE-05B`, with `EXR-SYSROOT-06` now unblocked and `EXR-PGCACHE-11B` still blocked behind `EXR-READ-11A`
- Immediate follow-on planning target: keep `EXR-PGCACHE-11B` explicitly fenced as a later follow-on until its board dependencies are actually satisfied
- Helper-surface state after the cleanup sweep:
  - `EXR-INOKEY-05A` now leaves only `ExfatInodeKey` in production code; the standalone `fs.rs` lookup wrapper was removed and exact opened-inode lookup is deferred to `EXR-MOUNT-09`
  - `EXR-INODE-05B` keeps constructors only; speculative metadata accessors were removed until a downstream component proves which facts need cross-module helpers
- Latest accepted dependencies relevant to this wave:
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-SBGEOM-15`
- Components in progress:
  - none
- Blocked components:
  - none

## Recent Decisions

- Resumed from the prior handoff by rereading:
  - `PROJECT_BRIEF.md`
  - `PROTOCOL.md`
  - `COMPONENT_INDEX.md`
  - `iron-ridge-20260401-1052-sbgeom-repair-and-inokey-prep.md`
- Revalidated that the current container is still `codex-asterinas-dev` and still lacks `/dev/kvm`.
- Narrowed the `EXR-INOKEY-05A` architect input set to:
  - role-scoped subagent rules from `.agents/protocol/`
  - accepted dependency artifacts from `EXR-CHAIN-03B` and `EXR-FILESET-04B`
  - full architect priors from:
    - `Microsoft-exFAT-spec.md`
    - `linux-exFAT-implementation-summary.md`
    - `ASTERINAS_ARCHITECT_PRIORS.md`
  - legacy Asterinas references for inode identity and opened-inode lookup in:
    - `kernel/src/fs/fs_impls/exfat/fs.rs`
    - `kernel/src/fs/fs_impls/exfat/inode.rs`
    - `kernel/src/fs/fs_impls/exfat/utils.rs`
- The architect packet is explicitly framed to keep `EXR-INOKEY-05A` about:
  - inode identity key derivation from on-disk location
  - root special-case identity
  - opened-inode-table lookup key helpers
- The architect packet explicitly forbids pulling inode metadata shaping, page-cache behavior, mount ownership, or directory iteration into this component.
- The `EXR-INOKEY-05A` architect artifact is accepted.
- Main-agent review adds one designer-side guardrail that the architect artifact implied but did not spell out strongly enough:
  - if `fs.rs` is touched for read-only lookup helpers, it must stay a minimal lookup-table wrapper or shared-state stub;
  - the designer must not introduce a real mount object, mount sequencing, or inode metadata shell in this component.
- The first designer pass was sent back because it leaked a concrete `ExfatInode` type into `EXR-INOKEY-05A`, which would have forced a premature inode shell before `EXR-INODE-05B`.
- The accepted designer revision fixes that leak by making the lookup surface payload-generic and read-only, so the component now stays dependency-safe relative to `EXR-INODE-05B`.
- The serial creator pass has landed and currently adds:
  - `inode.rs` with `ExfatInodeKey`,
  - `fs.rs` with a borrowed generic `OpenedInodeTable<'a, T>`,
  - `mod.rs` wiring for the new modules.
- The creator kept the component boundary clean:
  - no inode metadata shell,
  - no mount object,
  - no registry mutation API,
  - no page-cache or VFS behavior.
- The allowed compile-only verification command was attempted:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo check -p aster-kernel --lib'`
  - It still fails in pre-existing unrelated `ostd` dependency resolution (`acpi`, `x86_64`, `x86`, `tdx_guest`, `multiboot2`, `unwinding`) before giving exFAT-specific signal.
- The serial checker added local `#[ktest]` coverage in `inode.rs` and `fs.rs` and passed these filtered runs sequentially under `no-kvm` TCG:
  - `inode_key_preserves_packed_location_layout`
  - `root_inode_key_is_reserved`
  - `inode_key_rejects_offset_overflow`
  - `opened_inode_lookup_is_exact_match`
- The checker made one test-only repair during verification:
  - imported the local `#[ktest]` macro in the new test modules after the first filtered run exposed that omission.
- `02_designer_async.md` explicitly makes this component synchronous and ownership-light, with no dedicated concurrency implementation or concurrency-test phase required. The main-agent therefore treats the concurrency loop as intentionally empty and advances directly to review.
- Reviewer found no bounded code-quality issues and made no edits.
- Final checker reran the same four focused ktests after review under `no-kvm` TCG and they all passed again.
- `EXR-INOKEY-05A` is accepted.
- The protocol was tightened after `EXR-INOKEY-05A` acceptance to make prior precedence explicit:
  - Microsoft exFAT is now the normative semantic authority,
  - Linux exFAT is the preferred implementation reference when the spec leaves design room,
  - Asterinas-local priors are now explicitly limited to integration, interface, style, and testing context unless a packet records a deliberate local exception.
- This closes a real gap in the earlier protocol, which required packet-scoped priors but did not clearly forbid semantic drift back toward legacy Asterinas exFAT behavior by inertia.
- The protocol was tightened again after user review to enforce two new hard rules:
  - any temporary staging surface must be marked explicitly in code comments and in protocol artifacts, with a named future owner or removal condition;
  - any helper function, especially short field-exposing wrappers, must have an explicit packet- or artifact-backed reason to exist, and tests in the same module do not count as that proof.
- `EXR-INODE-05B` is now architected.
- The accepted architect boundary for `EXR-INODE-05B` is:
  - a read-only inode metadata shell built from accepted file-record facts, chain facts, and inode identity,
  - a synthetic root-shell special case using the reserved root identity and root-chain facts,
  - pure metadata accessors only.
- The architect artifact explicitly keeps these out of scope:
  - `PageCacheBackend`,
  - buffered I/O,
  - page-cache size coordination,
  - mount sequencing,
  - VFS-facing inode behavior,
  - directory-derived child accounting and parent propagation.
- The architect artifact also records `EXR-PGCACHE-11B` as planning-coupled but still blocked, which preserves the board order while protecting the ownership boundary during the upcoming designer pass.
- The `EXR-INODE-05B` designer set is accepted.
- The accepted designer set keeps the creator-facing scope narrow:
  - one ordinary constructor from accepted inode key, validated file-record facts, and validated chain facts,
  - one explicit root constructor,
  - pure metadata and chain accessors only.
- The accepted designer set explicitly excludes:
  - `PageCache`,
  - `PageCacheBackend`,
  - buffered I/O,
  - mount sequencing,
  - directory traversal,
  - registry mutation,
  - VFS method stubs.
- The `EXR-INODE-05B` serial creator pass has landed in `inode.rs`.
- The creator kept the component boundary clean:
  - `ExfatInodeMeta` is value-like and read-only,
  - one ordinary constructor plus one explicit root constructor,
  - pure metadata and chain accessors only,
  - no page-cache or live-inode behavior.
- The allowed compile-only verification command was attempted:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo check -p aster-kernel --lib'`
  - It still failed before reaching this component because of unrelated pre-existing `ostd` dependency/import errors (`acpi`, `x86_64`, `tdx_guest`, `multiboot2`, `unwinding`).
- The first serial checker pass found one narrow production compile defect:
  - the local `FatAttr` `bitflags!` form conflicted with the workspace macro surface, and `from_bits_retain` was unavailable.
- An advisor pass narrowed the repair to `inode.rs` only and explicitly excluded any metadata redesign or page-cache drift.
- The creator repair aligned `FatAttr` with the repo-compatible `bitflags!` pattern and replaced `from_bits_retain` with `from_bits_truncate`.
- The retry checker then passed these filtered ktests sequentially under `no-kvm` TCG:
  - `inode_meta_preserves_validated_file_record_facts`
  - `root_inode_meta_uses_explicit_synthetic_constructor`
  - `inode_meta_rejects_directory_length_mismatch`
  - `inode_meta_accessors_are_pure_read_only_views`
- `02_designer_async.md` explicitly makes `EXR-INODE-05B` a synchronous read-only value object, so the main-agent treats the concurrency loop as intentionally empty and advances directly to review.
- Reviewer found no bounded code-quality issues and made no edits.
- Final checker reran the same four focused inode-metadata ktests after review under `no-kvm` TCG and they all passed again.
- `EXR-INODE-05B` is accepted.
- A temporary reviewer sweep then audited helper surfaces across the refactor and drove a bounded cleanup:
  - removed the standalone `fs.rs` `OpenedInodeTable` staging wrapper and `mod fs;` wiring because no production caller existed yet;
  - removed speculative `ExfatInodeMeta` field accessors from `inode.rs`;
  - removed speculative `ExfatChain` getters from `fat.rs`;
  - removed unused dentry-slot setters from `fileset.rs`;
  - removed `cluster_size_in_sectors()` from `super_block.rs`;
  - gated `ExfatDentrySet::from_trusted_metadata`, `to_le_bytes`, and `update_checksum` to `#[cfg(ktest)]` and marked them as temporary write-side scaffolding pending a future writeback/builder owner.
- Post-cleanup serial verification under `no-kvm` TCG passed with these filtered commands:
  - `cargo osdk test inode_key_`
  - `cargo osdk test inode_meta_`
  - `cargo osdk test root_inode_meta_`
  - `cargo osdk test exfat_chain_`
  - `cargo osdk test fileset_`
  - `cargo osdk test cluster_translation_`

## Open Risks And Assumptions

- The legacy implementation uses `(cluster, offset)` plus a root sentinel hash to index opened inodes. That is useful prior art, but the architect still needs to decide whether the refactor should preserve that representation directly or wrap it in a more explicit key type.
- `EXR-INOKEY-05A` must not absorb the eventual inode metadata shell from `EXR-INODE-05B`, even though both touch the same legacy source area.
- Exact opened-inode lookup still needs a real owner later; after the helper cleanup it must reappear only inside a mount-owned state component, not as another freestanding wrapper.
- The deferred `UPCASE/NameHash` debt remains untouched in this wave.

## Recommended Next Actions

1. Re-read the board with `EXR-INODE-05B` accepted and the helper cleanup landed, then decide whether the next architect wave should still be `EXR-SYSROOT-06`.
2. Keep `EXR-PGCACHE-11B` fenced as a later follow-on until `EXR-READ-11A` and `EXR-MOUNT-09` make it executable.
3. When `EXR-MOUNT-09` eventually starts, remember that exact opened-inode lookup was intentionally deferred there; do not resurrect a freestanding wrapper earlier.
4. Continue treating all future checker/runtime work as serial in the current shared container.
5. Use both protocol tightenings in all future packets: prior precedence and helper/temporary-surface justification.

## Next Main-Agent Tasks

1. Re-read `COMPONENT_INDEX.md` with both `EXR-INOKEY-05A` and `EXR-INODE-05B` accepted and the helper cleanup landed, then select the next architect target by explicit dependency order rather than by local intuition.
2. Before dispatching the next wave, restate whether `EXR-SYSROOT-06` is now the next executable read-side component and whether any new parallel sibling exists.
3. Preserve both new protocol constraints in all downstream packets:
   - Microsoft/Linux prior precedence over legacy Asterinas semantics,
   - explicit justification for temporary staging surfaces and short helpers.
4. Do not forget that exact opened-inode lookup was deferred to `EXR-MOUNT-09`.
5. Do not forget the deferred `UPCASE/NameHash` debt when the `EXR-UPCASE-07A/07B` wave begins later.
