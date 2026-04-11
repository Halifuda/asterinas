<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Log

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` mount/open sequencing and root publication
- Status: `Pass with executable evidence`
- Author: Codex
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1605-checker-serial-packet.md`
- Checked spec:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`
- Pass kind: `serial checker retry`

## Scope

- Production files inspected:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- Local regression files exercised:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- Checker artifacts involved:
  - prior failing record: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/11_checker_serial.md`
  - repair record: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/13_creator_serial_repair.md`
  - current passing record: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/14_checker_serial_retry.md`

## Evidence

- `/dev/kvm` was visible before the retry loop via `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`.
- Actual test runs still emitted QEMU TCG CPU-feature warnings, so the recorded evidence is TCG-backed rather than KVM-backed.

## Commands Run

1. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test directory_engine_'`
2. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_inode_publication_returns_the_canonical_root_handle'`
3. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_special_case_stays_outside_the_ordinary_keyspace'`
4. `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_mount_sequence_installs_prerequisites_before_publishing_root'`

## Diagnostic Progression

- The earlier `11_checker_serial.md` record stopped at a generic environment diagnosis, but the retry loop used `/root/asterinas/qemu-serial.log` to prove the failures were real test failures, not a harness-only blocker.
- The sequence of concrete failures was:
  - `unexpected top-level directory dentry`
  - `compressed upcase table ended early`
  - `malformed FAT chain contents`
- The repair in `13_creator_serial_repair.md` resolved those failures without widening 22 beyond its intended owner boundary.

## Results

- The directory-side safety run `cargo osdk test directory_engine_` exited `0`.
- `cargo osdk test root_inode_publication_returns_the_canonical_root_handle` exited `0`.
- `cargo osdk test root_special_case_stays_outside_the_ordinary_keyspace` exited `0`.
- The final `cargo osdk test root_mount_sequence_installs_prerequisites_before_publishing_root` retry exited `0`.

## Filter Coverage Proof

- The exact root-publication suffix maps to `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs:1081`.
- The exact root-special-case suffix maps to `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs:1067`.
- The exact sequencing suffix maps to `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs:1099`.
- The `directory_engine_` filter is justified here because every local `DirectoryEngine` checker regression intentionally shares that prefix and the relevant set is source-bounded at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:305`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:353`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:395`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:434`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:463`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:490`

## Regression Coverage

- Repeated root publication returns the canonical published root handle.
- The root special case stays distinct from the ordinary opened-inode keyspace.
- Mount/open sequencing now reaches `DirectoryEngine` discovery, prerequisite installation, and canonical root publication in one owner-local path.
- The consumed `DirectoryEngine` prerequisite path tolerates the real root volume-label metadata entry without widening singleton policy beyond `Bitmap` and `Upcase`.

## Temporary Debug Use

- No temporary debug prints remain in the final source state.
- No debug-profile rerun was required in the final passing state.
- `qemu-serial.log` was inspected only as diagnostic evidence during failure classification.

## Conclusion

The repaired 22 implementation now passes the required checker coverage with executable evidence and is ready for review.
