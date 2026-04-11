<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` Upcase Table Ownership And Canonicalization Services
- Status: `SerialChecked`
- Author: Codex
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1220-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `serial`

## Scope of Review

Checked the owner-local upcase-table state and canonicalization services in `fs.rs` against the designer priors: validated publication, deterministic folding, and name hashing from folded UTF-16 bytes. Also checked the checker-owned local ktests added in `fs.rs` for malformed size rejection, checksum rejection, repeated folding stability, and hash stability without involving directory traversal or mount sequencing.

## Test Changes

- Added three local `#[ktest]` regressions in [`fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs).
- The checker-owned scenario comments are present on the new tests.
- No test relocation was needed.

## Findings

None.

## Verified Properties

- A well-formed synthetic upcase table is accepted and published once, while malformed size and checksum inputs are rejected before publication.
- The installed upcase table is the sole source of folding for `fold_utf16()`, and repeated folds of the same mixed-case input are deterministic.
- `name_hash()` and `name_hash_from_folded_utf16()` operate on the same folded UTF-16 canonical form, and case-equivalent names produce the same hash in the fixture.
- The checked behavior stays inside `fs.rs` and does not depend on directory traversal, mount sequencing, or inode discovery.
- `/dev/kvm` was present on the host, but the recorded QEMU runs reported TCG warnings, so the executed verification path used TCG.

## Unverified Properties

- No broader exFAT integration paths were exercised in this checker pass.
- I did not run the non-filtered kernel test suite.

## Recommendation

- Next owner: reviewer
- Reason: the serial checker obligations are satisfied and the remaining work is review-level confirmation of the same bounded slice.
- Blocking or non-blocking: non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required serial checker pass
