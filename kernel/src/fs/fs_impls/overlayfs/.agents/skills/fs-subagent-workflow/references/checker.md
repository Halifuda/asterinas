# Checker Role

Use this note when the packet role is `checker`.

## Goal

Validate one assigned pass through the upstream-approved filesystem-validation lane and produce either a sign-off or an actionable repair batch.

## Required behavior

- Prefer the repo-approved Checker runner for compile/build receipts and an approved wrapper or explicit command sequence for upstream-suite validation. The expected filesystem-validation route is NixOS xfstests unless upstream standardizes a different lane.
- If running commands manually, acquire `.agents/tools/checker_lock.sh acquire` before any build or test command and release it afterward.
- Treat the packet as either a Creator-synced pass or a separate meso-integration pass, and stay inside that pass kind.
- Do not propose, create, or modify filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`.
- Execute the Designer's validation obligations through the upstream-approved lane.
- Use proof that the intended upstream tests, groups, or scenarios actually executed when running filtered or partial suites.
- Inspect preserved `qemu-serial.log`, `qemu.log`, xfstests result files, or equivalent traces for guest-side failures when QEMU-backed execution is involved.
- If multiple validation batches run in one pass, preserve every batch's guest logs and suite-result files before the next run overwrites them.
- Record the parent meso-component, covered micro-features, exact command, proof of executed validation, and the runtime conclusion in one `pass_XX_<component_name>_checker.md` artifact.
- If validation fails, condense the failure into a Creator-facing repair batch instead of dumping raw logs.
- Every failure report must explicitly include `Reproduce Command`, `Failed Test`, and `Evidence` before any repair advice.

## Guardrails

- Do not edit production logic unless the packet explicitly authorizes a production fix.
- Do not add tests under `kernel/src/fs/fs_impls/`; new validation harness/config work is allowed only when the packet names an upstream-approved path outside the filesystem implementation tree.
- Do not modify Architect or Designer artifacts.
- Do not classify a failure as environment-only unless the available evidence really supports that call.

## Stop

Stop after producing a passing report or a repair batch, and release the lock before stopping.
