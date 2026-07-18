# Testing Guide

Use this note when checker execution or upstream-approved validation-harness work is in scope.

## Ownership

- Checker owns runtime verification by default.
- Checker must not add or recommend filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`.
- New filesystem behavior validation should use the upstream-approved external/system-level lane, currently expected to be NixOS xfstests unless upstream standardizes another route. For early `overlayfs` smoke, use the repository-local prebuilt-image guide `kernel/src/fs/fs_impls/overlayfs/.agents/XFSTESTS_PREBUILT_IMAGE_GUIDE.md` and the workspace-local wrapper explicitly named by the packet when that lane is assigned.
- Creator remains command-free unless the packet explicitly authorizes a compile-only exception.
- Respect the packet's pass kind: Creator-synced validation stays matched to one Creator Pass, while meso integration stays in its own Checker-owned pass.

## Default environment

- Preferred container: `codex-asterinas-dev`
- Mounted repository path: `/root/asterinas`
- Compile/build runner:

```bash
.agents/tools/checker_run.sh cargo-check --component <ID> --phase <PHASE>
.agents/tools/checker_run.sh make-kernel --component <ID> --phase <PHASE>
```

- Filesystem behavior validation:

Use the exact upstream-approved command shape named in the packet. The expected route is NixOS xfstests, so a valid receipt should record the NixOS/QEMU command, xfstests config, filesystem type proof, selected generic test IDs or groups, and result/notrun/fail files.

- Manual command shape for compile/build only, when the runner is not suitable:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo check -p aster-kernel --target x86_64-unknown-none'
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'
```

Use the exact command shape named in the packet when it differs.

## Proof and log rules

- Short filters are risky; prefer exact upstream test IDs, groups, or source-justified scenario names.
- `exit 0` alone is never enough evidence for filtered or partial upstream-suite runs.
- Record proof that the intended upstream tests actually executed in the Checker artifact.
- Treat preserved `qemu-serial.log`, `qemu.log`, xfstests result files, and equivalent traces as the primary guest-output sources when execution involved QEMU.
- Guest logs and result files may be overwritten by later QEMU or suite runs. Archive them after each validation batch before starting the next.
- On failure, the Checker artifact must still preserve the reproduce command, failed test, and concrete evidence verbatim.

## Classification

- Distinguish environment failure, build failure, suite setup failure, skipped/notrun classification, and test failure.
- Record whether KVM appeared available and whether the observed run used KVM or TCG when visible.
