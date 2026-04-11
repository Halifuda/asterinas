<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` Mount/Open Sequencing And Root Publication
- Status: `Reviewed`
- Author: Codex
- Date: `2026-04-11`
- Task packet: local main-agent review after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/14_checker_serial_retry.md`
- Reviewed implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- Pass kind: `review`

## Scope of Review

Reviewed the final 22 state after the checker-driven local repair, with focus on owner-boundary discipline, dependency hygiene, invariant expression, and whether the sibling `DirectoryEngine` repair stayed narrow instead of widening directory policy opportunistically.

## Production Edits

- None.
- No functional production changes were made in this review pass.

## Findings

None.

## Local Quality Notes

- `ExfatFs` remains the only mount/open owner; the review did not find a drift back toward a separate mount shell or scanner owner.
- `DirectoryEngine` still emits only validated file records plus raw `Bitmap` / `Upcase` singleton candidates. The new volume-label handling is a narrow skip, not a policy-widening generic-singleton escape hatch.
- `ExfatDentry::is_volume_label()` keeps the `0x83` classification local to dentry parsing and avoids spreading raw magic values through directory consumers.
- The tightened mount-ready helper in `fs.rs` now reuses the image's existing upcase slot and cluster, which removes the earlier arbitrary-cluster corruption risk from the checker fixture.

## Recommendation

- Next owner: main agent
- Reason: the reviewed slice is internally consistent and the retry checker already covers the runtime behavior that blocked acceptance.
- Blocking or non-blocking: non-blocking
- If this is the last reviewer pass before acceptance, state whether it was a required final reviewer or a previously recorded skip case: required reviewer pass
