<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Proposal: Post-28 Board Gap Audit

## Metadata

- Component ID: `WORKSPACE-ARCH-POST28`
- Title: `Post-EXR-DENTRY-WRITE-28 Board Re-Audit`
- Status: `Architected` (proposal artifact; does not itself enter implementation)
- Author: architect (delegated subagent)
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-POST28/20260413-1248-architect-packet.md`

## Problem Statement

The current owner-first board is strong through `EXR-DENTRY-WRITE-28`, and the post-28 tail already points at the right core owners:

- `EXR-NAMESPACE-29` on `ExfatInode`
- `EXR-WRITE-30` on `ExfatInode`
- `EXR-SYNC-31` on `ExfatFs`

That said, the board still leaves several architecturally real exFAT surfaces outside the current tail:

- direct I/O / `O_DIRECT`
- charset and name conversion beyond the upcase-table fold boundary
- backup-boot fallback and persistent boot-flag policy
- volume-label mutation
- admin/control surfaces such as trim/discard, forced shutdown, and FAT-attribute ioctls

The main question is not whether the core board can close buffered namespace/write/sync behavior. It can. The question is which omitted surfaces deserve their own stable owner-first rows and which should be closed explicitly as non-goals so the board does not drift into a compatibility bucket.

## Authority Used

This proposal is anchored in:

- the archived packet and its stop condition
- `COMPONENT_INDEX.md`
- `amber-delta-20260413-0725-dentry-write-closure.md`
- `ASTERINAS_ARCHITECT_PRIORS.md`
- `Microsoft-exFAT-spec.md`
- `linux-exFAT-implementation-summary.md`
- the authorized Linux source files under `/home/halifuda/linux/fs/exfat/`
- the current Asterinas code surfaces in `inode.rs`, `fs.rs`, and `kernel/src/fs/vfs/fs_apis/inode.rs`

The missing `EXR-SYNC-31/00_architect.md` file is a board gap, not a blocker. The recommendation below uses the current board and handoff as the authority for how far the sync tail should reach.

## Re-Audit Summary

### 1. `EXR-NAMESPACE-29` should stay inode-owned, but it should stop short of charset policy

`ExfatInode` is still the right final owner for `create`, `unlink`, `mkdir`, `rmdir`, and `rename`.
That remains a real inode-visible contract, and `DirectoryEngine`, `Allocator`, and opened-inode publication are still the right consumed owners.

What should change is the boundary around names:

- namespace mutation should not own charset conversion
- namespace mutation should not own upcase-table installation
- namespace mutation should not own volume-label control

The row should consume a separate name-conversion service and treat canonical name folding as a validated input, not as a local parsing concern.

### 2. `EXR-WRITE-30` should remain buffered-only

`ExfatInode::write_at`, `resize`, growth, and truncate are the right owner-first write tail.
The 2026-04-13 designer repair already clarified the call-local `ExfatInodeWriteState` model and the committed-allocation handoff.

What should change is explicit scope discipline:

- keep buffered regular-file mutation in `EXR-WRITE-30`
- keep `O_DIRECT` out of `EXR-WRITE-30`
- keep durable flush ordering out of `EXR-WRITE-30`

Direct I/O has its own alignment, read/zero-fill, and page-cache bypass semantics. It is real, but it is not the same boundary as buffered write.

### 3. `EXR-SYNC-31` should stay narrow

`EXR-SYNC-31` should be the filesystem-wide flush-ordering owner, not a control bucket.

It should own:

- dirty-producer flush ordering
- writeback sequencing across inode and filesystem state
- the persistence side of already-generated dirty state

It should not absorb:

- name conversion
- direct I/O
- boot fallback
- volume-label mutation
- trim/discard
- forced shutdown
- FAT-attribute ioctls

## Recommended Revised Tail

The board after `EXR-DENTRY-WRITE-28` should be reshaped as follows.

### Keep, but recut more narrowly

| ID | Recommended final owner | Landing form | Boundary note |
| --- | --- | --- | --- |
| `EXR-NAMESPACE-29` | `ExfatInode` | owner methods + narrow owner-private helpers | Keep namespace mutation inode-owned, but make name conversion an explicit consumed service rather than local policy. |
| `EXR-WRITE-30` | `ExfatInode` | owner methods + owner-private helpers | Keep buffered write/truncate/resize only. Make `O_DIRECT` an explicit downstream follow-on, not a hidden subcase. |
| `EXR-SYNC-31` | `ExfatFs` | owner methods | Keep flush ordering and persistence sequencing only. Do not fold admin controls into this row. |

### Add as new downstream modules

| Proposed ID | Functional goal served | Final owner | Landing form | Why it is a real boundary |
| --- | --- | --- | --- | --- |
| `EXR-CHARSET-32` | Charset and name conversion for VFS-visible exFAT names and label strings | `ExfatFs` | owner-private name-conversion service + validated name type | Linux keeps UTF-8/NLS conversion distinct from the upcase-table fold boundary; Asterinas needs a real codec boundary, not ad hoc conversion inside namespace mutation. |
| `EXR-DIRECT-33` | `O_DIRECT` regular-file read/write path and direct-I/O alignment rules | `ExfatInode` | owner methods + owner-private helpers | Direct I/O crosses a different contract from buffered writes: it bypasses the page cache and has separate zero-fill and alignment behavior. |
| `EXR-BOOT-34` | Backup-boot fallback and persistent boot-flag policy | `ExfatFs` | owner methods + mount/open policy helpers | Mount-time fallback and persistent volume-flag handling are recovery/mount concerns, not sync-policy concerns. |
| `EXR-VOLLABEL-35` | Volume-label read/write control surface | `ExfatFs` | owner methods + special root-metadata helper | Volume label mutation is a real filesystem-admin surface and a special root-directory metadata update, not namespace mutation. |

### Treat as explicit non-goals unless Linux-compatibility parity is being opened on purpose

| Item | Suggested disposition | Reason |
| --- | --- | --- |
| FAT attribute ioctls | explicit non-goal | Asterinas already has inode metadata APIs; the ioctl compatibility layer is not needed to close the core exFAT refactor tail. |
| `FITRIM` / discard maintenance | explicit non-goal for now | It is real, but it pulls in allocator/bitmap maintenance and device-policy detail that should not be hidden inside `EXR-SYNC-31`. |
| Forced shutdown ioctl | explicit non-goal for now | It is a real admin control, but it is a separate user-visible safety surface and should not be smuggled into the sync row. |

If the main agent later decides full Linux admin-parity is in scope, those non-goals should be reopened as separate owner-first rows rather than folded into `EXR-SYNC-31`.

## What This Means For The Current Tail

### `EXR-NAMESPACE-29`

Re-cut it so it is only about inode-owned namespace mutation.

It should:

- consume the new name-conversion service
- consume `EXR-UPCASE-20`
- consume `EXR-DIR-OPS-23`
- consume `EXR-DENTRY-WRITE-28`
- consume `EXR-ALLOC-27`
- publish canonical children through `ExfatFs`

It should not:

- parse charset policy locally
- own volume-label controls
- absorb sync or persistence ordering

### `EXR-WRITE-30`

Keep it as the buffered-write owner for regular files.

It should:

- own buffered `write_at`
- own `resize`
- own growth and truncate behavior
- consume committed allocation results
- keep the page cache coherent

It should not:

- add `O_DIRECT`
- add a writeback queue
- add sync policy
- take over inode-admin ioctls

### `EXR-SYNC-31`

Keep it as the persistence-ordering owner only.

It should be the row that remembers "what must be flushed and in what order," not "what administrative action user space asked for."

That distinction matters because boot fallback, label mutation, and forced shutdown are control-path decisions, while sync is a flush-path decision.

## Collision Zones And Sequencing

The likely collision point remains `inode.rs`.

That file will host:

- `EXR-NAMESPACE-29`
- `EXR-WRITE-30`
- `EXR-DIRECT-33`

Those rows should not be treated as file-parallel implementation lanes. They will need serialized creator waves, because they share the same inode carrier, the same publication boundary, and much of the same owner-private helper region.

`fs.rs` is the second likely collision point for:

- `EXR-CHARSET-32`
- `EXR-BOOT-34`
- `EXR-VOLLABEL-35`
- `EXR-SYNC-31`

The main-agent should expect those rows to share owner-private filesystem state, not separate service objects.

## Final Recommendation

The cleanest post-28 shape is:

1. keep `EXR-NAMESPACE-29`, but narrow it to inode-owned namespace mutation plus a consumed name-conversion service
2. keep `EXR-WRITE-30`, but make it buffered-only and keep `O_DIRECT` out
3. keep `EXR-SYNC-31`, but narrow it to persistence ordering only
4. add `EXR-CHARSET-32` for name/charset conversion
5. add `EXR-DIRECT-33` for `O_DIRECT`
6. add `EXR-BOOT-34` for backup-boot fallback and boot-flag policy
7. add `EXR-VOLLABEL-35` for volume-label control
8. leave FAT attribute ioctls, trim/discard, and forced shutdown as explicit non-goals unless the main agent wants a compatibility-parity pass

That gives the main agent a board that stays owner-first, avoids hidden helper drift, and keeps the remaining Linux/Asterinas integration surfaces visible instead of smuggled into the core data-path rows.

