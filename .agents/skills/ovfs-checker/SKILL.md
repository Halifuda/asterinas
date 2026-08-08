---
name: ovfs-checker
description: Use when running or triaging authorized overlayfs xfstests validation in the codex-asterinas-dev Docker container.
---

# Overlayfs Checker

This is the Checker specialization of `$ovfs-subagent`. Use it only for a
main-agent-authorized runtime validation packet. The packet still defines the
component, pass, and covered micro-features; this skill supplies the verified
overlay xfstests execution lane.

## Plugin boundary

Do not load, invoke, or follow any `superpowers:*` skill for this task. The
workspace configuration disables the superpowers plugin; use this skill and
the repository-local Checker protocol instead. Report any packet that
requires superpowers as a protocol conflict to the main agent.

## Dispatch delivery (platform-verified 2026-08-08)

This specialization is dispatched through the V2 lane defined in
`$ovfs-main` and PROTOCOL.md §1.3: the User Dispatch Turn names the packet
path; read the packet file directly and treat it as the sole contract. The
spawn payload and the NEW_TASK header are NOT readable on this platform;
verify identity with `list_agents` (the running non-root agent whose name
matches the dispatched task_id).

Runtime authorization — the approved command lane, runlist, disk size, and
evidence destination — is conveyed by the packet file alone. Do not accept
runtime authorization from spawn payload, followup messages, or conversation
text. Followup/send messages are not readable and are not a valid channel for
repair rounds; each round is a new User Dispatch Turn.

## Execution environment

Run the actual kernel and xfstests workflow inside the existing privileged
container:

```text
codex-asterinas-dev
```

The repository is `/root/asterinas` in that container. Confirm the container
is available and preserve the current run's logs and suite results before
starting a rerun. Use this skill's serialized command lane and any external
lock explicitly supplied by the packet; do not run competing QEMU jobs.

## Default artifacts and storage

The default paths for a run are:

- `/root/asterinas/qemu.log`: host-visible QEMU guest output;
- `/root/asterinas/qemu-serial.log`: host-visible serial output;
- `/root/asterinas/test/initramfs/build/xfstests_test.img`: generated test
  image;
- `/root/asterinas/test/initramfs/build/xfstests_scratch.img`: generated
  scratch image;
- `/opt/xfstests/results/`: guest-side xfstests result files, which are not
  persistent in the container after QEMU exits unless explicitly copied out.

On the host, the first two paths are normally
`/home/ayd/asterinas/qemu.log` and `qemu-serial.log`. The two generated image
files are commonly 6 GiB or larger and are recreated or overwritten by later
runs. Do not put those large images in Git; record their size and SHA-256 in
the Checker receipt instead.

For the old implementation baseline, use
`kernel/src/fs/fs_impls/overlayfs/.agents/components/old-ovfs-baseline-test/`.
For later runs, use a new timestamped directory under the packet-authorized
`kernel/src/fs/fs_impls/overlayfs/.agents/components/<component-id>/` path.
The overlayfs `components/` tree is intentionally Git-ignored: keep local
evidence there, but do not stage receipts, raw logs, or large image metadata
from it unless the main agent explicitly changes that policy.

## Verified overlay xfstests startup

The known working command shape is:

```bash
docker exec -w /root/asterinas codex-asterinas-dev \
  make run_kernel AUTO_TEST=conformance RELEASE=1 MEM=12G \
  CONFORMANCE_TEST_SUITE=xfstests \
  XFSTESTS_FS_TYPE=overlay \
  XFSTESTS_DISK_SIZE=6G \
  XFSTESTS_RUNLIST=/opt/xfstests/overlay/run_list/short.list
```

The overlay runner and configuration live at:

- `test/initramfs/src/conformance/xfstests/run_xfstests.sh`
- `test/initramfs/src/conformance/xfstests/overlay/`

The test image must be at least 6 GiB because later cases create a large
file. The previously verified image was 8 GiB ext2. Keep the lower, upper,
and work directories on the backing ext2 filesystem and prove that the guest
mounted the intended overlay filesystem before classifying results.

## Run selection

- Start with a bounded smoke run using the configured overlay `short.list`.
- For a packet requesting a PR runlist, pass its exact runlist through
  `XFSTESTS_RUNLIST`; record the requested case count and the cases actually
  executed.
- A full overlay list currently contains more cases than the PR subset. Do
  not substitute the full list for a requested subset without recording that
  change in the receipt.
- Later cases may fail independently or hang on the legacy overlayfs. A
  failure count is not the acceptance criterion; successful startup, execution
  evidence, preserved logs, and accurate classification are.

## Evidence and hang handling

Before each rerun, archive the prior guest serial/QEMU log and xfstests result
files under the packet-authorized component receipt directory. Record:

- the exact host and container command;
- the resolved filesystem type and image size;
- the runlist and cases observed in the output;
- guest logs and suite result paths;
- pass, failure, not-run, and timeout/hang classifications;
- a reproducible command for every actionable failure.

Archive the raw `qemu.log` and `qemu-serial.log` before they are overwritten.
If `/opt/xfstests/results/` is available while the guest is still running,
copy the relevant result files before teardown; otherwise classify the result
from the preserved QEMU/serial output and state that guest result files were
not host-persistent. Snapshot the exact runlist and filesystem config beside
the logs. After copying, verify the archive hashes and image metadata.

Only after confirming that no QEMU process is still running and the receipt is
complete may the exact generated files
`test/initramfs/build/xfstests_test.img` and
`test/initramfs/build/xfstests_scratch.img` be removed to reclaim space. Do not
delete the whole build directory or logs as a substitute for targeted cleanup.

If QEMU or the old overlayfs hangs, stop that single run using the approved
container cleanup procedure, retain its logs first, and do not start another
run until the execution lane is clear. A hang after earlier cases have run is
evidence about the hang location, not evidence that the suite never started.

## Reporting

Write the required Checker receipt/report under the packet's authorized
`kernel/src/fs/fs_impls/overlayfs/.agents/components/<component-id>/` path. Keep the
Creator-synchronized scope exact. Separate infrastructure/startup failures
from filesystem behavior mismatches, and send the main agent the original
diagnostics without reinterpretation. Never add filesystem-local ktests or
test fixtures as part of diagnosis.
