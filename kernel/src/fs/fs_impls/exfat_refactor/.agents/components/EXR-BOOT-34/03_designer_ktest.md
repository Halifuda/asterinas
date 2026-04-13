<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-BOOT-34`
- Title: `ExfatFs` Boot-Policy Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1421-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/00_architect.md`

## Purpose

Define the minimum checker-owned regressions needed to prove that boot-region fallback and persistent boot-flag policy stay owned by `ExfatFs`, that mount/open consumes the policy result without re-owning it, and that sync later consumes only the published dirty boot intent.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs` and any owner-private test helpers that the future creator adds there
- Helper touch: owner-private helpers may be added only if they are needed to construct trusted boot-policy fixtures

## Required Coverage

### Scenario 1: Primary boot facts remain the default trusted source

- Test intent:
  - Confirm that a healthy mount path trusts the validated primary boot facts when no owner-private fallback override is published.
- Suggested test shape:
  - Build a mounted `ExfatFs` fixture from a known-good exFAT image.
  - Publish the policy result.
  - Complete mount/open and request the ready root.
- Assertions:
  - The mount/open path succeeds.
  - The published policy result selects the primary source.
  - Root publication still happens through the `EXR-FS-OPEN-22` owner boundary.

### Scenario 2: Fallback selection is explicit and owner-private

- Test intent:
  - Confirm that fallback only wins when the owner-private policy result selects it.
- Suggested test shape:
  - Build a synthetic mismatch fixture with trusted primary facts and an owner-private validated fallback candidate.
  - Publish the policy result.
  - Exercise the mount/open consumer.
- Assertions:
  - The policy result selects the fallback source only when the owner-private decision says so.
  - The test does not need a second parser or a separate recovery worker.
  - The selected source remains stable for later mount/open calls on the same mounted instance.

### Scenario 3: VolumeDirty becomes persistent boot intent

- Test intent:
  - Confirm that `VolumeDirty` is preserved as boot-region writeback intent and does not disappear into generic inode metadata.
- Suggested test shape:
  - Seed the owner-private policy state with a dirty boot flag.
  - Mount the filesystem and then trigger sync-side observation.
- Assertions:
  - The published policy result carries dirty boot intent forward.
  - Later sync code can observe the intent.
  - The dirty intent is not rewritten as a different owner type.

### Scenario 4: ClearToZero remains a pre-mutation requirement

- Test intent:
  - Confirm that `ClearToZero` is remembered until a later mutating path clears it.
- Suggested test shape:
  - Publish a policy result that carries `ClearToZero`.
  - Complete mount/open without performing a mutation.
  - Inspect the owner-private snapshot.
- Assertions:
  - Mount/open does not clear the bit by accident.
  - The bit remains visible as a pre-mutation guard.
  - The result is still distinct from `VolumeDirty`.

### Scenario 5: PercentInUse remains observational

- Test intent:
  - Confirm that changing only `PercentInUse` does not change the trusted boot source or the persistent boot intent.
- Suggested test shape:
  - Vary only the percent-in-use observation across two fixtures.
  - Publish the boot-policy result for each fixture.
- Assertions:
  - The selected boot source is unchanged.
  - The dirty boot intent is unchanged.
  - No space-accounting owner is implied by the observation.

### Scenario 6: Sync consumes intent, not policy logic

- Test intent:
  - Confirm that later sync code sees the published dirty boot intent and does not re-run the boot-source decision.
- Suggested test shape:
  - Publish a boot-policy result that carries dirty intent.
  - Call `FileSystem::sync()` twice on the same mounted filesystem.
- Assertions:
  - The first call can observe the dirty boot intent.
  - The second clean call remains success-only.
  - Sync does not become a second boot-policy owner.

## Observability

- These tests should inspect mount/open success, the published boot-policy snapshot, and the presence or absence of persistent boot intent.
- They should treat `VolumeDirty` and `ClearToZero` as boot-region outputs, not as inode metadata.
- They should not add direct-I/O coverage, volume-label coverage, trim/discard coverage, or admin ioctl coverage.
- They should not introduce a second parser, a background recovery worker, or any new public boot-policy API.

## Minimal Checker Obligation

The checker must include regressions proving that:

- the primary boot facts remain the default trusted source,
- fallback selection is an explicit owner-private decision,
- `VolumeDirty` survives as persistent boot intent,
- `ClearToZero` survives as a pre-mutation guard,
- `PercentInUse` stays observational,
- and sync consumes the already-published dirty boot intent rather than the decision logic itself.

## Exit Condition

The ktest plan is complete when a future checker can implement it entirely in local `fs.rs` tests and can verify that boot-region policy stays above validated boot facts, below mount/open orchestration, and separate from filesystem-wide sync ordering.
