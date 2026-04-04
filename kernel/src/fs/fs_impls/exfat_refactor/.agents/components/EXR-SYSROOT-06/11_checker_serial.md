<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-SYSROOT-06`
- Title: Root-Directory System-Entry Scanner
- Status: `SerialChecked`
- Author: `checker`
- Date: `2026-04-04`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`

## Scope of Review

Reviewed the checker-owned validation slice for the root-directory system-entry scanner against the packet, `COMMON_SUBAGENT.md`, `CHECKER.md`, `TESTING_GUIDE.md`, and the component architect/designer artifacts.

Added local `#[ktest]` coverage in `sysroot.rs` for:

- mixed-root discovery with unrelated file content,
- duplicate bitmap rejection,
- missing bitmap rejection,
- missing upcase rejection,
- malformed bitmap start-cluster rejection,
- wrong-kind secondary rejection,
- truncated root-record rejection.

## Verification

- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test sysroot::tests'` exited `0`.
- QEMU printed TCG feature warnings during the run, so the observed runtime mode was TCG rather than KVM.
- The command completed after the kernel test image and ISO were built, booted, and returned without a build or test failure.

## Findings

No blocking findings.

## Verified Properties

- The scanner preserves bitmap and upcase discovery facts, including location, start cluster, byte size, and the `UPCASE` checksum.
- The scanner rejects duplicate, missing, malformed, wrong-kind, and truncated root-directory records at the boundary.
- The coverage stays local to `sysroot.rs` and does not require mount bootstrap, page-cache, or async behavior.

## Recommendation

- Next owner: `main-agent`
- Reason: checker coverage and execution evidence are complete for the packet scope.
- Blocking or non-blocking: `Non-blocking`
