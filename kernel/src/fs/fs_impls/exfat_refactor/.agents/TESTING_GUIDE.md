<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Testing Guide

This note explains how testing should be executed for the exFAT multi-agent workflow.
It focuses on the Asterinas container-based path that has already been validated in this workspace.

## 1. Role Decision

The workflow should not introduce a separate dedicated test-writer role by default.

Instead:

- the `creator` owns production implementation,
- the `checker` owns verification,
- and the `checker` is also the default owner of writing or updating targeted tests.

This is the better default because:

1. test obligations come directly from specification verification,
2. regression tests are usually easiest to write when the failing behavior is fresh in the checker context,
3. a separate test-only role would add one more handoff boundary without improving accountability.

A separate test author should only be introduced if testing itself becomes a large independent stream of work.

Designer specifications may still state test obligations, but those obligations are checker-owned.
Creators should ignore spec instructions to write `#[ktest]` coverage unless the main agent explicitly overrides the default workflow.

## 2. What Kinds of Tests Exist In Asterinas

Asterinas has two relevant test styles:

1. ordinary Rust `#[test]` for non-kernel crates that can run with `cargo test`,
2. kernel-mode `#[ktest]` for kernel and OSTD code, executed with `cargo osdk test`.

For `exfat_refactor` under `kernel/src/fs/fs_impls/exfat_refactor/`, the important test style is `#[ktest]`.

Checker-owned `#[ktest]` code does not need to live in one central `mod.rs`.
The preferred layout is:

1. place a ktest next to the module it validates using `#[cfg(ktest)] mod tests`,
2. use a small shared `test_support.rs` or similar test-only module for reusable fixtures,
3. keep `mod.rs` free of unrelated test piles unless there is a concrete reason to centralize.

Test-only helpers should follow the same default layout:

1. keep helpers inside the local `mod tests` when only that module's tests use them,
2. move reusable fixtures or builders into `test_support.rs` or another clearly test-only module when multiple test modules need them,
3. avoid adding `#[cfg(ktest)]` methods inside the production `impl` body unless the helper is a documented cross-module test surface that cannot live in a test-only module yet.

If such an exception is unavoidable, the code comment should say why it cannot live under `mod tests` or `test_support.rs` and should name the future owner or removal condition.

Each checker-owned `#[ktest]` should also include a short comment that says what scenario the test sets up and what behavior it is intended to confirm.
These comments do not need to restate every line of the test body, but they should make the purpose obvious to a reader who is scanning the file.

## 3. Current Recommended Execution Environment

The currently validated environment is the long-lived Docker container:

```text
codex-asterinas-dev
```

It is started from:

```text
asterinas/asterinas:0.17.1-20260317
```

with the repository mounted at:

```text
/root/asterinas
```

This path has already been proven to support:

- installing or building `cargo-osdk`,
- `make kernel`,
- `cargo osdk test <TESTNAME>`,
- QEMU-based ktest execution.

During the parallel-refactor phase, the legacy default `mount -t exfat` path is not the primary validation target for `exfat_refactor`.
The default strategy is to validate the new module with targeted ktests first, and then with dedicated integration tests once the new implementation becomes mountable on its own terms.

## 4. Verified Basic Commands

### Build the kernel in the container

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'
```

### Run a selected ktest from the kernel crate

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test new_exfat'
```

This path has already been exercised successfully.

When the main agent delegates test execution to a checker, the task packet should repeat the containerized command shape explicitly instead of assuming the checker will remember it.
For example, the packet should say that commands must use:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test ...'
```

rather than a host-side `cargo osdk test ...` invocation.

## 5. How Test Selection Actually Works

`cargo osdk test [TESTNAME]` does not need to run the entire ktest population.
It supports filtered execution.

The important detail is that the filter behaves as a **test-path suffix match**, not as a full regular expression engine.

Conceptually:

- the runner forms a path like `module_path::function_name`,
- the provided `TESTNAME` is split on `::`,
- the test runs if the provided path is a suffix of the full test path.

This means the following are possible:

```bash
cargo osdk test new_exfat
cargo osdk test test::new_exfat
cargo osdk test exfat::test::new_exfat
```

The shorter the suffix, the greater the chance of accidental collisions.

So the recommended practice is:

1. use the shortest suffix only for quick local probing when the name is unique,
2. use a longer and more explicit suffix when recording a reproducible checker command.

## 5A. How To Prove The Filter Actually Hit The Intended Tests

When a checker records a filtered `cargo osdk test <TESTNAME>` run, it must also record why that filter is known to hit the intended tests.

This is required because a green exit status alone is not sufficient evidence of coverage. In this workspace, `cargo osdk test` can still exit `0` even when the filter matches nothing.

The checker must therefore provide one of these proof forms:

1. source-backed suffix proof:
   - inspect the local `#[ktest]` names,
   - record the exact function-name suffix or longer `module_path::function_name` suffix used in the command,
   - state why that suffix is unique enough for the intended coverage;
2. output-backed proof:
   - record command output that explicitly names the executed tests.

Broad module-like filters such as `fs::tests` are not good enough on their own unless the checker also records why they cannot silently miss or over-match the intended cases.

For `cargo osdk test`, the authoritative local implementation points are:

- `osdk/deps/test-kernel/src/lib.rs`, where the runner checks the whitelist against the test path as a suffix;
- `ostd/libs/ostd-test/src/lib.rs`, where OSDK forwards the requested whitelist into the runner.

The practical default in this workflow should be:

1. use exact `#[ktest]` function names, or a longer explicit suffix, for recorded checker commands;
2. cite the corresponding source locations in the checker artifact;
3. do not claim coverage from `exit 0` alone.

## 6. How Checkers Should Use Tests

The checker should think in three layers:

1. existing tests that already cover the obligation,
2. selected ktests that should be executed for this component,
3. missing tests that should be added before acceptance.

The checker should usually:

- run the smallest relevant filtered ktest set first,
- add or update a targeted ktest when behavior changed and coverage is missing,
- check and record whether KVM appears available before drawing conclusions from runtime-sensitive behavior,
- rerun only the relevant filtered tests during iteration,
- leave broad or full-suite runs for later checkpoints when needed.

Role restrictions still apply while using this guide:

- the main agent, architect, designer, and advisor should not run kernel build or test commands;
- creator passes are command-free by default; compile-only kernel commands should be treated as explicit packet-level exceptions, never as a routine requirement;
- the checker owns `cargo osdk test`, `make ktest`, and other runtime verification commands.
- a checker may prepare tests and reports before execution, but command-producing verification should enter only after acquiring the shared execution lock through `.agents/tools/checker_lock.sh`.

The concrete lock flow is:

1. run `.agents/tools/checker_lock.sh acquire --component ... --phase ... --command ... --retry-seconds 60 --wait-budget-seconds ...`,
2. run the assigned verification command after acquisition succeeds,
3. run `.agents/tools/checker_lock.sh release` when that command-producing stage is finished.

## 7. Minimum Test Obligations By Change Type

### Mount or boot-region changes

The checker should prefer tests that validate:

- superblock loading,
- boot-region validation,
- root inode construction,
- bitmap and upcase-table discovery during mount.

### Directory or lookup changes

The checker should prefer tests that validate:

- lookup behavior,
- readdir behavior,
- name handling,
- dentry-set parsing and validation,
- create/delete interactions.

### FAT, bitmap, allocation, truncate, or write-path changes

The checker should prefer tests that validate:

- contiguous versus FAT-chain handling,
- allocation and free behavior,
- file size and allocated-size invariants,
- truncation behavior,
- read/write visibility and data integrity.

### Bug fixes

The checker should normally add a regression ktest for any confirmed bug fix.

## 8. Practical Execution Advice

### Check KVM first

Before running heavier ktests, the checker should determine whether the current environment appears to expose KVM.

In the validated container workflow, a simple practical check is:

```bash
docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'
```

Interpretation:

- if `/dev/kvm` is present and accessible, KVM may be available to QEMU,
- if `/dev/kvm` is absent, the checker should expect TCG fallback,
- even when `/dev/kvm` exists, the checker should still watch the QEMU output for signs that the run fell back to TCG.

The checker should record both:

1. the preflight environment fact,
2. the observed runtime mode when it is visible from QEMU output.

### Prefer filtered tests during iteration

Do not start with full-suite ktests unless the task explicitly requires it.
For most component work, a small filtered run is faster and gives clearer feedback.

### Run kernel tests sequentially

Do not run multiple `cargo osdk test`, `make ktest`, or other QEMU-producing commands in parallel from this workflow.
Tooling-level directory conflicts can produce misleading failures that are unrelated to the component under review.

More generally, in the current workflow the repository checkout and Docker container are shared mutable execution state.
That means command-producing subagent work should be treated as serial by default, not only QEMU-backed tests.
If the main agent wants true parallel command execution, it should first arrange isolated worktrees, isolated build directories, and isolated container or runtime state.

### Expect the first run to be expensive

The first `cargo osdk test` in a fresh environment may spend substantial time:

- compiling the test base crate,
- building the kernel test image,
- building the QEMU bootable ISO.

Later runs are usually cheaper.

### TCG warnings are not automatically fatal

If QEMU runs without KVM acceleration, it may print TCG CPU-feature warnings.
Those warnings do not automatically mean the test is invalid.
They matter mainly for speed, not for basic functional confirmation.
However, they do matter for portability and for future sessions on different machines, so the checker should explicitly record when a result came from a TCG-backed run.

### Distinguish build failure from test failure

There are three distinct failure classes:

1. environment failure: `cargo-osdk`, toolchain, or QEMU setup is broken,
2. build failure: the kernel test image does not compile,
3. test failure: the ktest runner executes and reports failing tests.

Checkers should state clearly which class occurred.

## 9. What To Record In Checker Artifacts

Every checker pass should record:

- the exact test command or commands used,
- whether the run was filtered or broad,
- whether KVM appeared available before the run,
- whether the observed run looked like KVM or TCG,
- which tests were added or updated,
- which spec obligations were confirmed by executable checks,
- which obligations remain untested,
- whether the result was an environment failure, build failure, or test failure.

## 10. Current Status Of The Testing Path

At the time of writing, the following has been validated in the container workflow:

1. `make kernel` succeeds in `codex-asterinas-dev`.
2. `cargo osdk test new_exfat` succeeds from `/root/asterinas/kernel`.
3. QEMU boot and selected exFAT ktest execution are usable in the containerized path.

That is sufficient to treat container-based filtered ktest execution as the default testing path for this workspace.
