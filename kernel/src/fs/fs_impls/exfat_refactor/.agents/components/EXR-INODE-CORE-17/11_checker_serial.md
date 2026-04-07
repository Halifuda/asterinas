<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: EXR-INODE-CORE-17
- Title: Inode Carrier And Metadata Owner
- Status: `SerialChecked`
- Author: checker
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1052-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial`

## Scope of Review

Checked `inode.rs` against `01_designer_core.md`, `03_designer_ktest.md`, `10_creator_serial.md`, the VFS `Inode` / `InodeIo` trait surface, and the local `ExfatFs` integration surface allowed by the packet.

## Test Changes

Added one local `#[ktest]` in `inode.rs`:

- `inode_carrier_snapshots_metadata_and_rejects_temporary_seams`

The test has a short scenario comment and covers trusted dentry/chain snapshot construction, `metadata()` coherence with dedicated accessors, weak `fs()` owner recovery, and explicit `EOPNOTSUPP` rejection for `read_at()`, `write_at()`, `resize()`, `set_mode()`, `set_owner()`, and `set_group()`.

The recorded filter suffix is source-backed: `rg -n "inode_carrier_snapshots_metadata_and_rejects_temporary_seams" kernel/src/fs/fs_impls/exfat_refactor` found exactly `kernel/src/fs/fs_impls/exfat_refactor/inode.rs:284`.

## Findings

### Finding

- Severity: Blocking verification environment failure
- Location: Docker container `codex-asterinas-dev`
- Description: The filtered ktest could not execute because `docker exec` reported that the named container is not running.
- Violated spec clause or expected behavior: `CHECKER.md` and `TESTING_GUIDE.md` require executable evidence from the packet's Docker command form when command-producing verification is assigned.
- Reproduction or reasoning:
  - Lock acquired with:
    - `.agents/tools/checker_lock.sh acquire --component EXR-INODE-CORE-17 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'" --retry-seconds 60 --wait-budget-seconds 1800`
  - KVM preflight attempted under the lock:
    - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
    - Result: `Error response from daemon: container cdc951c6672504dbbc3568609420f831b7db8e475798d8ca062b3d035c8b64b1 is not running`
  - Filtered ktest attempted under the lock:
    - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'`
    - Result: `Error response from daemon: container cdc951c6672504dbbc3568609420f831b7db8e475798d8ca062b3d035c8b64b1 is not running`
  - Lock released with:
    - `.agents/tools/checker_lock.sh release`
    - Result: `status = "unlocked"`

No implementation finding was found in the static `inode.rs` review.

## Verified Properties

- Source inspection found no `InodeKey`, inode cache helper, page-cache helper, directory operation, namespace mutation, or hidden durable write-side mutation in `inode.rs`.
- `ExfatInode` stores a `Weak<ExfatFs>` and upgrades it in `fs()` without creating a strong filesystem ownership edge.
- `metadata()` and the dedicated metadata accessors read the same copied `Metadata` snapshot.
- The data-path seams include the required temporary comment naming `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26`.
- `resize()`, `set_mode()`, `set_owner()`, and `set_group()` reject with `EOPNOTSUPP` rather than mutating hidden writeback state.

## Unverified Properties

- The added ktest was not compiled or executed because the Docker container was not running.
- KVM availability and QEMU runtime mode were not observed because the KVM preflight could not enter the container.
- Runtime proof that the filter executed the intended ktest is unavailable; only the source-backed unique suffix proof is available in this pass.

## Recommendation

- Next owner: main agent
- Reason: Restore or restart the packet's required `codex-asterinas-dev` execution environment, then rerun the filtered ktest command under the checker lock.
- Blocking or non-blocking: Blocking for executable acceptance evidence, non-blocking for the static implementation review.
- This was the required serial checker pass; executable verification is incomplete due to an environment failure.
