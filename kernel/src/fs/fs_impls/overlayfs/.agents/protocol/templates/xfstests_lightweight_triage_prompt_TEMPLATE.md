<!-- SPDX-License-Identifier: MPL-2.0 -->

# Lightweight xfstests Triage Prompt: `{phase}`

Use the temporary xfstests lightweight triage protocol:
`kernel/src/fs/fs_impls/overlayfs/.agents/protocol/XFSTESTS_LIGHTWEIGHT_TRIAGE.md`.

## Scope

- **Component ID:** `{component_id}`
- **Phase:** `{phase}`
- **Target bucket expected by main agent:** `{expected_bucket}`
- **Target case(s):** `{generic_cases}`
- **Baseline seconds, if performance case:** `{baseline_seconds_or_na}`
- **Allowed production logging edits:** `{none_or_logging_only_files}`
- **Receipt output path:** `{receipt_path}`

## Required Harness Command

Run only this command shape, adjusting only explicitly bracketed placeholders:

```sh
kernel/src/fs/fs_impls/overlayfs/.agents/tools/xfstests_run.sh {case_or_batch_or_direct} {case_args} --phase {phase} {extra_harness_args}
```

Do not hand-write the long `make run_kernel` command. Do not widen the case
list. Do not edit official scheduler state. If production logging is authorized,
edit only the named logging files and do not change logic, state, lock behavior,
allocation policy, error mapping, or validation semantics.

## Work Steps

1. Run the harness command.
2. Inspect the receipt root produced by the harness.
3. Inspect `qemu.log`, `qemu-serial.log`, runlist proof, execution proof, and
   result files.
4. Write the receipt below.
5. Escalate instead of guessing if required evidence is missing.

## Receipt To Produce

```markdown
<!-- SPDX-License-Identifier: MPL-2.0 -->

# Lightweight xfstests Triage Receipt: `{phase}`

## Identity

- **Component ID:** `{component_id}`
- **Phase:** `{phase}`
- **Target case(s):** `{generic_cases}`
- **Receipt root:** `{receipt_root}`
- **Harness command:** `{exact_command}`

## Execution Proof

- **Runlist or direct command proof:** `{path_and_key_lines}`
- **Filesystem type proof:** `{path_and_key_lines}`
- **Intended case execution proof:** `{path_and_key_lines}`
- **Exit status:** `{status}`

## Log Scan

- **qemu.log:** `{panic_deadlock_tcg_stall_failure_scan}`
- **qemu-serial.log:** `{panic_deadlock_tcg_stall_failure_scan}`
- **xfstests result files:** `{result_file_summary}`

## Classification

- **Bucket:** `{PASSABLE|PASSABLE_PERF_REGRESSION|ABOVE_EXFAT_CONFIRMED|EXFAT_CANDIDATE|HARNESS_OR_ENV|INCONCLUSIVE}`
- **Smallest observed failing boundary:** `{boundary_or_na}`
- **Owner reasoning:** `{short_evidence_based_reasoning}`

## Performance

- **Baseline seconds:** `{baseline_or_na}`
- **Observed seconds:** `{observed_or_na}`
- **Delta seconds:** `{delta_or_na}`
- **Performance verdict:** `{verdict_or_na}`

## Escalation

- **Escalation needed:** `{yes_or_no}`
- **Reason:** `{reason_or_na}`
- **Next exact action:** `{action}`
```
