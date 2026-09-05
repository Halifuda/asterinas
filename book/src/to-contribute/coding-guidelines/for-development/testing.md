# Testing

### Add regression tests for every bug fix (`add-regression-tests`) {#add-regression-tests}

When a bug is fixed,
a test that would have caught the bug should accompany the fix.
Include a reference to the issue number
in a comment so future readers
can recover the original context.

See also:
PR [#2962](https://github.com/asterinas/asterinas/pull/2962).

### Test user-visible behavior, not internals (`test-visible-behavior`) {#test-visible-behavior}

Tests should validate observable, user-facing outcomes.
Prefer testing through public APIs
rather than exposing internal constants in test code.

Name tests after the behavior or specification concept being verified,
not after internal implementation details.
Using kernel-internal names in user-space regression tests
creates unnecessary coupling.

See also:
PR [#2926](https://github.com/asterinas/asterinas/pull/2926).

### Use assertion macros, not manual inspection (`use-assertions`) {#use-assertions}

Use language- or framework-provided assertion helpers
instead of printing values and manually inspecting output.
Assertions provide clear failure messages
and make tests self-checking.

See also:
PR [#2877](https://github.com/asterinas/asterinas/pull/2877)
and [#2926](https://github.com/asterinas/asterinas/pull/2926).

### Clean up resources after every test (`test-cleanup`) {#test-cleanup}

Always clean up resources after a test:
close file descriptors, unlink temporary files,
and call `waitpid` on child processes.
Leftover resources can cause flaky failures
in subsequent tests.

```c
// Good — cleanup after use
int fd = open("/tmp/test_file", O_CREAT | O_RDWR, 0644);
// ... test logic ...
close(fd);
unlink("/tmp/test_file");
```

See also:
PR [#2926](https://github.com/asterinas/asterinas/pull/2926)
and [#2969](https://github.com/asterinas/asterinas/pull/2969).

### Run kernel-mode unit tests with `make ktest` (`run-kernel-unit-tests`)

Kernel-mode unit tests (`#[cfg(ktest)]` modules) execute inside QEMU,
not on the host, so a passing exit code alone is not evidence:
a crate whose guest produces no output must be reported as
not attributable instead of inferred passing.

Run the whole suite with `make ktest`;
CI runs it as `make ktest NETDEV=tap`.
The target invokes `cargo osdk test` for every workspace
default-member crate. With OSDK 0.18.x the only test selection is the
positional `TESTNAME`, which matches a test-path suffix: its last
`::`-segment must equal the test function's name, and `--package` or
`--ktests` flags do not exist, so tests are selected by exact name
rather than by crate.

Note that a crate linking the kernel command-line parser keeps its
early console disabled unless the guest command line carries
`earlycon`. Since `cargo osdk test` passes an empty command line,
such crates run their tests silently — pass the console key
explicitly (for example
`make ktest CARGO_OSDK_TEST_ARGS='--kcmd-args=earlycon ...'`)
whenever per-test output is the evidence you need.
