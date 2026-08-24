<!-- SPDX-License-Identifier: MPL-2.0 -->

# xfstests Lightweight Triage Protocol

This is a temporary, narrow protocol layer for high-volume xfstests follow-up
after the June 2026 filesystem implementation/refactor baseline split.

It does **not** replace the normal delegated Checker role in `PROTOCOL.md`.
The output of this lane is a triage receipt, not official acceptance. The main
agent may accept a receipt into official state only after checking that the
receipt preserves the required evidence and does not make a production repair
decision beyond its evidence.

## 1. Purpose

Use this lane for repetitive xfstests work that is too judgment-dependent for a
script but mechanical enough to run on a low-cost model when the packet is
evidence-shaped:

- rerunning passable cases and recording performance deltas;
- confirming that a VFS / PageCache / MM / BIO candidate failure is still
  above `overlayfs`;
- collecting focused logging receipts before a main-agent or formal Checker
  decision.

The intended operators are low-cost agents such as Haiku-class or
Qwen-27b-Dense-class models. The prompt must be small and evidence-shaped.

## 2. Authority Boundary

Lightweight triage agents may:

- run the packet-authorized `$ovfs-checker` container command with a bounded
  case or run list;
- add production logging when the packet explicitly authorizes logging edits
  and names the files;
- inspect preserved `qemu.log`, `qemu-serial.log`, xfstests result files,
  runlist receipts, and reproduce commands;
- write one triage receipt under `.agents/components/<component-id>/` or a
  packeted temporary receipt directory.

Lightweight triage agents must not:

- mark a Creator / Checker pass accepted;
- modify production logic code for a repair;
- change control flow, data structures, persisted state, lock behavior,
  allocation policy, error mapping, or validation semantics;
- widen the target case list or owner scope;
- edit `SYSTEM_BLUEPRINT.md`, `PASS_SLICING.md`, or main-agent handoffs;
- introduce filesystem-local tests under `kernel/core/src/fs/fs_impls/`;
- claim an above-`overlayfs` owner unless the logs show the smallest
  observed boundary outside the filesystem implementation/refactor implementation.

Production logging means temporary diagnostic calls or marker text only. It may
touch production files because the failing behavior often appears only in the
production path, but it must be reversible and behavior-preserving. If the next
step requires non-logging logic changes, escalate out of the lightweight lane.

## 3. Harness Rule

Use the verified `$ovfs-checker` container command unless the packet
explicitly says otherwise. For a bounded case or batch, provide a packeted
run list through `XFSTESTS_RUNLIST`.

Required command shape:

```sh
docker exec -w /root/asterinas codex-asterinas-dev \
  make run_kernel AUTO_TEST=conformance RELEASE=1 MEM=12G \
  CONFORMANCE_TEST_SUITE=xfstests XFSTESTS_FS_TYPE=overlay \
  XFSTESTS_DISK_SIZE=6G \
  XFSTESTS_RUNLIST=/opt/xfstests/overlay/run_list/<packet-run-list>
```

The Checker owns the serialized command lane, exact runlist, and receipt
archive. Do not widen the case list or run outside `codex-asterinas-dev`.

## 4. Result Buckets

Choose exactly one bucket:

- `PASSABLE`: the intended case executed, passed, and produced no guest panic,
  deadlock, stall, or `[not run]` marker.
- `PASSABLE_PERF_REGRESSION`: the intended case passed but is slower than the
  packeted baseline or exceeds the packeted threshold.
- `ABOVE_EXFAT_CONFIRMED`: the failure reproduces and the smallest observed
  failing boundary is outside `kernel/core/src/fs/fs_impls/overlayfs/`.
- `EXFAT_CANDIDATE`: the failure reproduces and the smallest observed failing
  boundary is inside or immediately entered from `overlayfs`.
- `HARNESS_OR_ENV`: the intended case did not execute, devices / filesystem
  type were wrong, QEMU was stale, or result files are insufficient.
- `INCONCLUSIVE`: evidence is preserved but does not justify a stronger bucket.

## 5. Required Evidence

Every receipt must include:

- exact harness command;
- receipt root;
- runlist content or decoded direct command;
- proof that `FSTYP=OverlayFs`;
- proof that the intended case(s) executed or a precise reason they did not;
- exit status;
- `qemu.log` / `qemu-serial.log` scan for panic, deadlock, TCG, stall,
  `[not run]`, and xfstests failure markers;
- result bucket;
- escalation decision.

Performance receipts must also include:

- packeted baseline seconds;
- observed seconds;
- delta seconds;
- whether the delta is functionally meaningful under the packet's threshold.

Above-owner receipts must also include:

- first failing log line or smallest stable stall boundary;
- immediate caller / callee chain if visible;
- why that boundary is above `overlayfs`;
- what evidence is missing if the verdict is only a candidate.

## 6. Escalation Triggers

Escalate to the main agent or a formal Checker before continuing if:

- production code repair is needed;
- logging would touch files outside the packet;
- owner boundary crosses multiple subsystems;
- logs contradict the expected bucket;
- the harness fails before producing `qemu.log` / `qemu-serial.log`;
- the case is flaky across two same-command reruns;
- the receipt would change official state rather than merely propose a bucket.

## 7. Output Template

Use `.agents/protocol/templates/xfstests_lightweight_triage_prompt_TEMPLATE.md`
for the prompt and receipt shape unless the packet provides a stricter format.
