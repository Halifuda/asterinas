# EXR-BOOT-34 Architect Boundary

## Recommended Unit

Make `EXR-BOOT-34` the smallest owner-first `ExfatFs` policy unit that decides:

- which boot-region source to trust at mount/open time when primary and backup facts disagree,
- when the boot-region state must be persisted back as `VolumeDirty`,
- when the boot-region state must be persisted back as `ClearToZero`,
- and how `PercentInUse` is treated as a policy input or a non-owning observation.

This row should own the policy layer above validated boot facts, not the parser, not the generic recovery path, and not filesystem-wide flush ordering.

## Ownership Shape

### Keep inside `EXR-BOOT-34`

- Owner-private `ExfatFs` helpers that choose the boot-region source for later mount/open use.
- Owner-private policy helpers that answer whether boot-region flags should be published as dirty or cleared-to-zero.
- The stable decision surface that later sync code can consume as a boot-region dirty output, without deciding the flush order here.
- The `PercentInUse` stance for this row, limited to policy interpretation and not expanded into recovery or admin control.

### Keep outside `EXR-BOOT-34`

- `EXR-BOOT-01` validated primary boot parsing.
- `EXR-FS-OPEN-22` mount/open sequencing and any orchestration that merely calls the policy surface.
- `EXR-SYNC-31` filesystem-wide flush ordering and any global sync scheduling.
- Name conversion, volume-label control, direct I/O, trim/discard, forced shutdown, and FAT-attribute ioctls.

## Stable Boundary Recommendation

The stable architectural split is:

1. `EXR-BOOT-01` proves the primary boot facts.
2. `EXR-BOOT-34` decides whether those facts or the backup path should win for mount/open policy, and whether persistent boot-region flags must be marked for later writeback.
3. `EXR-FS-OPEN-22` performs the mount/open sequence and consumes the policy result.
4. `EXR-SYNC-31` later consumes the dirty output, but does not own the decision that created it.

This is the smallest real unit that keeps boot fallback and boot-flag persistence together without turning the row into a generic recovery shell.

## Policy Notes

- Treat `VolumeDirty` and `ClearToZero` as persistent boot-region policy outputs owned by `ExfatFs`.
- Treat `PercentInUse` as a row-local stance only if it affects the boot-region decision surface; otherwise keep it observational and do not widen the row into space accounting.
- Do not collapse boot fallback into a sync bucket. The decision that produces a dirty boot-region output belongs here, while the ordering of the eventual writeback belongs to `EXR-SYNC-31`.

## Likely Creator-Slice Guidance

Later creator work should look for one narrow owner-private surface in `fs.rs` or a nearby `ExfatFs` impl block that:

- accepts validated boot-region facts,
- returns a mount-time boot-source decision,
- returns persistent-flag intent for `VolumeDirty` and `ClearToZero`,
- and emits a dirty boot-region marker without performing sync ordering.

The main collision zones in `fs.rs` are any code paths that already handle mount/open sequencing, generic flush, or admin-style boot manipulation. Those paths should consume this row’s decision, not absorb it.

## Exit Boundary

Stop after this architect boundary is recorded. Do not define tests, do not prescribe lock ordering, and do not expand the row into recovery, sync, or admin control.
