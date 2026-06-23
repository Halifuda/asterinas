<!-- SPDX-License-Identifier: MPL-2.0 -->

# xfstests Guidelines for `exfat_refactor`

This note is the scheduler-facing guide for moving `exfat_refactor` validation
from the historical NixOS prebuilt-image harness to upstream Asterinas xfstests.

## Current Baseline

The branch now contains upstream `main` through `42d38f9af388c28c2dda8cb4d6e62de465f23e8e`.
That upstream includes two changes that materially change the old xfstests
classification:

- `f7045ff78` supports unaligned block device I/O. This should remove the old
  guest `/dev/vdb` blocker where `pread(fd, 64, 0)` returned `EINVAL`.
- `33fe74e82` adds regression coverage for unaligned block device I/O.

Upstream also provides a standard xfstests conformance entry:

- `make run_kernel AUTO_TEST=conformance CONFORMANCE_TEST_SUITE=xfstests`
- `.github/workflows/test_x86.yml` runs the short xfstests list.
- `.github/workflows/test_xfstests_full.yml` runs the full list on schedule or
  manual dispatch.
- `test/initramfs/src/conformance/xfstests/` contains the runner, runlists, and
  blocklist.

## Important Limitation

The upstream xfstests scaffold is currently ext2-oriented:

- `test/initramfs/Makefile` creates `xfstests_test.img` and
  `xfstests_scratch.img` with `mkfs.ext2`.
- `test/initramfs/src/conformance/xfstests/run_xfstests.sh` mounts both devices
  with `mount -t ext2`.
- `test/initramfs/src/conformance/xfstests/local.config` exports `FSTYP=ext2`
  and `MKFS_OPTIONS="-F"`.
- `test/initramfs/nix/conformance/xfstests.nix` includes ext2/xfs tooling but
  not `exfatprogs`.

Therefore, do not run the upstream xfstests target against `exfat_refactor`
unchanged and treat the result as an exFAT verdict. It is currently an ext2
test lane.

## Migration Direction

Prefer adapting the upstream initramfs conformance lane over expanding the
historical `test/nixos/tests/xfstests` harness. The old NixOS harness remains
useful as a diagnostic fallback because it can reuse preserved images and export
guest logs, but it was built to work around missing upstream support and the old
block-device read blocker.

The target shape is:

1. Keep upstream's default ext2 behavior unchanged for CI.
2. Add explicit xfstests parameters for non-ext2 filesystems.
3. Add an `exfat_refactor` mode that uses exFAT on-disk formatting tools but
   mounts through the Asterinas filesystem name `exfat_refactor`.
4. Run only the first small named batch before considering broad runlists.

## Required Adaptation Points

Keep these changes outside `kernel/src/fs/fs_impls/exfat_refactor/` unless a
real production bug is discovered by the run.

1. Add xfstests filesystem knobs.

   Recommended Make variables:

   - `XFSTESTS_FSTYP ?= ext2`
   - `XFSTESTS_MKFS ?= mkfs.ext2`
   - `XFSTESTS_MKFS_OPTIONS ?= -F`
   - `XFSTESTS_FSCK ?= fsck.ext2`

   For `exfat_refactor`, the intended values are:

   - `XFSTESTS_FSTYP=exfat_refactor`
   - `XFSTESTS_MKFS=mkfs.exfat`
   - `XFSTESTS_MKFS_OPTIONS=`
   - `XFSTESTS_FSCK=fsck.exfat`
   - `XFSTESTS_MOUNT_OPTIONS=`

   Do not infer `mkfs.$FSTYP` from `FSTYP` for `exfat_refactor`; the mount type
   is `exfat_refactor`, while the host/guest formatter is the standard exFAT
   formatter.

2. Pass the knobs to the guest.

   `Makefile` already passes `XFSTESTS_RUNLIST`, `XFSTESTS_TEST_DEV`, and
   `XFSTESTS_SCRATCH_DEV` as kernel command-line arguments for xfstests. Extend
   the same path for `XFSTESTS_FSTYP`, `XFSTESTS_MKFS`,
   `XFSTESTS_MKFS_OPTIONS`, and `XFSTESTS_FSCK`.

3. Parameterize the image creation.

   `test/initramfs/Makefile` should keep ext2 as the default but use the new
   formatter knobs for `XFSTESTS_TEST_IMAGE` and `XFSTESTS_SCRATCH_IMAGE`.
   `exfat_refactor` should create exFAT-formatted raw images with the standard
   exFAT tools.

4. Parameterize the runner.

   `run_xfstests.sh` should use `XFSTESTS_FSTYP` instead of hard-coded `ext2`
   for its preflight mounts. It should also make the xfstests environment agree
   with the same value.

5. Fix `local.config`.

   The current static `local.config` hard-codes ext2 and ext2-specific mkfs
   options. Replace it with either:

   - shell parameter expansion that defaults to ext2 but honors exported
     `XFSTESTS_*` variables, or
   - a generated config written by `run_xfstests.sh` before invoking `./check`.

   `exfat_refactor` needs helper compatibility because upstream xfstests may
   call `mkfs.$FSTYP` or `fsck.$FSTYP`. The minimal approach is to put wrappers
   or symlinks named `mkfs.exfat_refactor` and `fsck.exfat_refactor` earlier in
   `PATH`, forwarding to `mkfs.exfat` and `fsck.exfat`.

6. Add `exfatprogs` to the xfstests initramfs runtime.

   `test/initramfs/nix/conformance/xfstests.nix` must include `exfatprogs` for
   guest-side `mkfs.exfat` and `fsck.exfat`.

## First Real Test Batch

After the adaptation above, the next Checker should run a deliberately small
batch, not `short.list` or `full.list`.

Use this initial set:

```text
generic/001
generic/007
generic/013
```

Keep `generic/023` out of the first run because the historical receipt already
classified it as a symlink capability `notrun`, not an exFAT logic failure.

The first run must prove that:

- the guest mounts `TEST_DEV` and `SCRATCH_DEV` as `exfat_refactor`;
- the formatter/checker wrappers resolve inside the guest;
- the selected xfstests names actually execute;
- xfstests result files and QEMU logs are archived before any rerun;
- a failure is classified from result files plus `qemu.log` / serial logs, not
  from exit status alone.

## Suggested First Command Shape

The exact wrapper should be owned by a Checker pass, but the command should stay
close to upstream's lane:

```sh
make run_kernel \
  AUTO_TEST=conformance \
  CONFORMANCE_TEST_SUITE=xfstests \
  RELEASE=1 \
  MEM=12G \
  XFSTESTS_DISK_SIZE=2G \
  XFSTESTS_FSTYP=exfat_refactor \
  XFSTESTS_MKFS=mkfs.exfat \
  XFSTESTS_MKFS_OPTIONS= \
  XFSTESTS_FSCK=fsck.exfat \
  XFSTESTS_MOUNT_OPTIONS= \
  XFSTESTS_RUNLIST=/opt/xfstests/exfat_refactor_s1a.list
```

If the Makefile cannot safely pass an empty `XFSTESTS_MKFS_OPTIONS` value, use a
runtime defaulting rule in `run_xfstests.sh` or `local.config` instead of adding
shell quoting hacks to the Checker command.

## Direct Diagnostic Command Mode

The upstream initramfs xfstests runner also supports a diagnostic mode for
localizing a mounted-filesystem failure without running upstream `./check`.
This mode is for Checker-owned diagnosis only; it is not an xfstests pass/fail
receipt.

Inputs:

- `XFSTESTS_DIRECT_COMMAND`
- `XFSTESTS_DIRECT_COMMAND_B64`
- `XFSTESTS_DIRECT_WORKDIR`

Default empty direct-command variables preserve ordinary xfstests behavior.
When either command variable is set, `run_xfstests.sh` prepares the same
runtime wrappers and `local.config`, mounts TEST and SCRATCH with
`XFSTESTS_FSTYP` / `XFSTESTS_MOUNT_OPTIONS`, exports `TEST_DIR` and
`SCRATCH_MNT`, changes to `XFSTESTS_DIRECT_WORKDIR` (default
`/opt/xfstests/test`), executes the decoded command via `/bin/sh -c`, prints
the command and exit status into the run log, syncs, and unmounts on exit.

Checker should prefer `XFSTESTS_DIRECT_COMMAND_B64` for any non-trivial
command. Raw kernel command-line arguments are not a safe transport for spaces,
semicolons, pipes, redirections, or shell variables. A typical command
preparation is:

```sh
cmd='set -x
mkdir -p "$TEST_DIR/permname.direct/a"
cd "$TEST_DIR/permname.direct/a"
/opt/xfstests/src/permname -c 4 -l 6 -p 1 || echo "permname returned $?"
find . -maxdepth 1 -print
find . -print | wc -l'
b64=$(printf "%s" "$cmd" | base64 -w0)
```

Then pass `XFSTESTS_DIRECT_COMMAND_B64="$b64"` and
`XFSTESTS_DIRECT_WORKDIR=/opt/xfstests/test` to the same
`make run_kernel AUTO_TEST=conformance CONFORMANCE_TEST_SUITE=xfstests ...`
lane. Checker receipts for this mode must explicitly say they are diagnostic
direct-command receipts and must still preserve `run.log`, fresh QEMU logs, the
checker-lock acquire/release files, and the exact decoded command.

## Protocol Boundary

The main agent should not run the QEMU-backed xfstests batch directly. Create a
Checker packet that:

- names `xfstests_classification_20260509` as the validation wave;
- points to this guide;
- requires use of `.agents/tools/checker_lock.sh` or a wrapper that acquires it;
- requires preserved logs under `.agents/checker-runs/xfstests_classification_20260509/`;
- forbids adding tests under `kernel/src/fs/fs_impls/`;
- routes production failures back as repair batches rather than ad hoc fixes.

## Historical Fallback

The old prebuilt-image assets remain useful only for diagnosis:

- runner: `.agents/tools/xfstests_prebuilt_runner.sh`
- images: `.agents/xfstests/images/`
- logs: `.agents/xfstests/logs/`
- NixOS harness: `test/nixos/tests/xfstests/`

Use this lane only if the upstream conformance path cannot yet express the
needed exFAT setup, or if a preserved-image comparison is required. New primary
coverage should converge on upstream `test/initramfs/src/conformance/xfstests`.
