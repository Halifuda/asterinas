<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `iron-ridge`
- Date: 2026-04-01 10:52 CST
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Updated after the `EXR-CHAIN-03B` and `EXR-FILESET-04B` acceptance wave

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260317`
- Working path: `/root/asterinas` inside the container, `/home/halifuda/asterinas` on the host
- Container name, if any: `codex-asterinas-dev`
- KVM status: current exFAT checker evidence still says `no-kvm`, so observed ktest runs are TCG-backed.
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test exfat_chain_walks_contiguous_chain_and_reports_offsets'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests::fileset_valid_construction_round_trip_serialization'`
- Known environment blockers:
  - No `/dev/kvm` inside the current container.
  - Shared-worktree and shared-container command execution is not parallel-safe; build, OSDK, and QEMU-producing work should be treated as serial unless explicit isolation is prepared first.

## Current Project State

- Current goal: Continue the refactor under the tightened subagent-dispatch protocol after landing the first chain and file-record value layers.
- Current phase: `CHAIN` and `FILESET` are now accepted; the next ready-now implementation slice is `EXR-INOKEY-05A`, followed by `EXR-INODE-05B`.
- Active or next component:
  - immediate implementation wave: `EXR-INOKEY-05A`
  - immediate planning wave: `EXR-INODE-05B` architect and spec once `EXR-INOKEY-05A` is shaped
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
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

## Open Risks And Assumptions

- The new protocol split only helps if the main agent actually avoids `fork_context: true` for ordinary subagents and sends minimal task packets.
- `EXR-CHAIN-03B` and `EXR-FILESET-04B` are intentionally narrow value layers. Future creators or reviewers must not let them absorb inode identity, namespace iteration, mount policy, or write-side allocation behavior.
- Future planning still needs scrutiny around `EXR-CREATE-12` and `EXR-WRITE-13`, and possibly whichever component grows during detailed design.
- One reviewer packet accidentally forbade all commands instead of only build or test commands, which produced a blocked review artifact on the first `EXR-CHAIN-03B` reviewer attempt. The pass was immediately reissued with explicit allowance for read-only inspection commands.

## Recommended Next Actions

1. Start `EXR-INOKEY-05A` with delegated architect and designer packets, again using `fork_context: false`.
2. Prepare `EXR-INODE-05B` only after `EXR-INOKEY-05A` is concrete enough that inode metadata boundaries stay narrow.
3. Keep checker and final-checker runtime verification serial; do not let multiple command-producing subagents overlap in the current container.

## Resume Checklist

- Read `PROJECT_BRIEF.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Verify the environment summary above still matches reality.
