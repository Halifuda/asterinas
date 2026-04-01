<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `iron-ridge`
- Date: 2026-04-01 10:52 CST
- Author: main-agent
- Covered hours: approximately `2.8` hours, from `2026-04-01 10:52 CST` to `2026-04-01 13:40 CST`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Updated through the `EXR-SBGEOM-15` repair closure, helper-surface protocol tightening, and `EXR-INOKEY-05A` prep

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260317`
- Working path: `/root/asterinas` inside the container, `/home/halifuda/asterinas` on the host
- Container name, if any: `codex-asterinas-dev`
- KVM status: current exFAT checker evidence still says `no-kvm`, so observed ktest runs are TCG-backed.
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test cluster_translation_rejects_invalid_clusters'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test cluster_range_validation_uses_half_open_semantics'`
- Known environment blockers:
  - No `/dev/kvm` inside the current container.
  - Shared-worktree and shared-container command execution is not parallel-safe; build, OSDK, and QEMU-producing work should be treated as serial unless explicit isolation is prepared first.

## Current Project State

- Current goal: Continue the refactor under the tightened subagent-dispatch protocol after landing the first chain and file-record value layers, with the superblock geometry cleanup accepted and the next implementation wave narrowed around `EXR-INOKEY-05A`.
- Current phase: `CHAIN`, `FILESET`, and the superblock-geometry repair are now accepted. The next ready-now implementation slice is `EXR-INOKEY-05A` under the revised task-board narrowing.
- Active or next component:
  - immediate implementation wave: `EXR-INOKEY-05A`
  - immediate planning wave: `EXR-INODE-05B` and the new page-cache slice after `EXR-INOKEY-05A` is shaped
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
  - `EXR-SBGEOM-15`
  - `EXR-FATVAL-03A`
  - `EXR-DENTRY-04A`
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
- Components in progress:
  - none
- Blocked components:
  - none

## Recent Decisions

- The workflow protocol has been split into a main-agent scheduler protocol plus role-scoped subagent packet rules under `.agents/protocol/`.
- Task packets must now declare read scope, write scope, forbidden files, stop condition, and explicit execution environment.
- Command-producing subagent work is treated as serial by default in the current shared checkout and shared container model.
- The protocol now also requires main-agent handoffs to be maintained during the wave, to record covered hours, and to end with explicit next-main-agent tasks. It also now treats semantically overlapping helper surfaces as a design smell unless a canonical helper and a clear justification are recorded.
- A narrow cleanup component, `EXR-BOOTTYPE-14`, was added and accepted to type the validated boot-sector boundary via `ValidatedBootSector`.
- The component graph was tightened:
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
  - `EXR-INOKEY-05A`
  - `EXR-INODE-05B`
- `EXR-CHAIN-03B` is accepted. It adds the read-only `ExfatChain` value layer, explicit empty-chain handling, contiguous and FAT-backed walking, offset mapping, and checker-owned ktests for traversal behavior.
- `EXR-FILESET-04B` is accepted. It adds the validated `ExfatDentrySet` value layer, raw-name aggregation, checksum verification and recomputation, ordered serialization, and checker-owned ktests for malformed ordering and checksum behavior.
- The `CHAIN` and `FILESET` architect/designer work was delegated with `fork_context: false` and bounded read/write sets. The serial command-producing roles were kept one-at-a-time in the shared container.
- This wave did not show the earlier kind of scope overreach: no subagent edited `COMPONENT_INDEX.md` or source files outside its assigned write set. The only workflow blemish was one reviewer packet that was too strict and had to be reissued.
- A prior-informed reviewer/checker pass found one confirmed geometry bug and one deferred semantic debt:
  - `super_block.rs` currently stores `ClusterCount + 2` in a field named like a count and then uses it as an inclusive upper bound, so `cluster_count + 2` is wrongly accepted as a valid data-cluster id.
  - `fileset.rs` currently computes `NameHash` from raw UTF-16 code units. This is knowingly provisional and must be revisited once the upcase-table service exists.
- Architect review with higher-priority Asterinas priors tightened the remaining plan:
  - `EXR-INOKEY-05A` now depends on `EXR-CHAIN-03B` and stays strictly about inode identity and opened-inode-table lookup.
  - `EXR-INODE-05B` is metadata-only and must not absorb `PageCacheBackend`.
  - Page-cache backend behavior is promoted into its own planned slice, `EXR-PGCACHE-11B`.
- `EXR-SBGEOM-15` is now accepted. It fixes the old `ClusterCount + 2` confusion by keeping `num_clusters` as the raw data-cluster count, introducing explicit bound helpers, making those helpers canonical in `super_block.rs` and `fat.rs`, and rerunning the focused geometry ktests under `no-kvm` TCG.
- Cluster ids `0` and `1` are now called out explicitly as reserved in the refactor code comments, so the local code no longer relies on readers remembering that fact only from the Microsoft spec.

## Open Risks And Assumptions

- The new protocol split only helps if the main agent actually avoids `fork_context: true` for ordinary subagents and sends minimal task packets.
- `EXR-CHAIN-03B` and `EXR-FILESET-04B` are intentionally narrow value layers. Future creators or reviewers must not let them absorb inode identity, namespace iteration, mount policy, or write-side allocation behavior.
- The deferred `NameHash` issue must not be forgotten: when `EXR-UPCASE-07A/07B` land, the file-set hash path needs a follow-up repair so it hashes an up-cased filename instead of the current raw UTF-16 units.
- Future planning still needs scrutiny around `EXR-CREATE-12` and `EXR-WRITE-13`, and whichever component starts to absorb page-cache or mount-wide shared-state ownership during detailed design.
- One reviewer packet accidentally forbade all commands instead of only build or test commands, which produced a blocked review artifact on the first `EXR-CHAIN-03B` reviewer attempt. The pass was immediately reissued with explicit allowance for read-only inspection commands.

## Recommended Next Actions

1. Start `EXR-INOKEY-05A` with delegated architect and designer packets, again using `fork_context: false` and the revised dependency on `EXR-CHAIN-03B`.
2. Prepare `EXR-INODE-05B` and `EXR-PGCACHE-11B` together at the planning level so page-cache ownership does not slide back into the inode shell by accident.
3. Keep checker and final-checker runtime verification serial; do not let multiple command-producing subagents overlap in the current container.

## Next Main-Agent Tasks

1. Launch the `EXR-INOKEY-05A` architect pass with a packet that includes the relevant Microsoft, Linux, and Asterinas priors, especially the opened-inode-table and on-disk-location identity context.
2. After `EXR-INOKEY-05A` is architected, delegate the split designer artifacts and keep the creator-facing scope narrow enough that the component does not absorb inode metadata or page-cache behavior.
3. Keep `EXR-INODE-05B` and `EXR-PGCACHE-11B` coupled at the planning stage so page-cache ownership remains explicit.
4. Do not forget the deferred `UPCASE/NameHash` debt when `EXR-UPCASE-07A/07B` begins; the current `fileset.rs` hash is knowingly provisional.

## Resume Checklist

- Read `PROJECT_BRIEF.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Verify the environment summary above still matches reality.
