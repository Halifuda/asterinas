<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1306-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1306-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-CHARSET-32`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 13:06 CST`

## Goal

- Produce the split designer artifact set for `EXR-CHARSET-32` so later creator work can implement `ExfatFs`-owned external-name/label conversion without guessing about validated converted-value shape, `EXR-UPCASE-20` handoff boundaries, or namespace/label consumer obligations.

## Architectural Unit Context

- Functional goal: `ExfatFs` charset and external-name conversion boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal conversion service plus validated converted-name/value types
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`

## Required Resolution Questions

- Specify the smallest filesystem-owned conversion surface that accepts Asterinas `&str` names or label strings and produces validated UTF-16 values for exFAT consumers.
- State exactly how the row hands validated converted-name values to `EXR-NAMESPACE-29` and converted-label values to `EXR-VOLLABEL-35`.
- Keep `EXR-UPCASE-20` as the only owner of fold/hash over UTF-16 units; this row must stop before canonical fold/hash behavior.
- State the external contract explicitly as UTF-8 text from Asterinas VFS and treat Linux-style byte-string / optional NLS policy as a non-goal rather than a hidden second interface.
- Define narrow creator and checker obligations so later work does not guess where UTF-8 validation ends and later exFAT canonicalization begins.
- State serialization and repeated-call expectations for the conversion service without creating a generic Unicode helper module.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-POST28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/linux/fs/exfat/exfat_fs.h`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/namei.c`
- `/home/halifuda/linux/fs/exfat/file.c`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary as authoritative.
- The stable external contract is Asterinas `&str` input, not Linux byte-string / NLS policy.
- `EXR-UPCASE-20` remains the only owner of fold/hash over UTF-16 units.
- This row owns UTF-8 external-string validation and conversion into validated UTF-16 values only.

## Integration Prior Inputs

- `inode.rs` currently calls `name.encode_utf16()` directly in lookup paths and uses `String::from_utf16()` for visible names; this row exists to stop later namespace/label work from repeating ad hoc conversion in consumers.
- `EXR-NAMESPACE-29` is blocked on this row and should consume a validated converted-name value rather than raw `&str`.
- `EXR-VOLLABEL-35` should consume the same conversion boundary for label strings, but it should not consume fold/hash semantics.
- `fs.rs` already owns the upcase table and name-hash services; designer work must keep the new conversion boundary before that owner line, not across it.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the active `EXR-SYNC-31` designer lane because the write sets are disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatFs`.
- Reject drift into a generic text helper module, fold/hash service, or namespace mutation shell.

## Temporary Interfaces And Exit Plan

- Do not authorize Linux-style optional NLS or byte-string APIs as a second stable contract in this designer pass.
- Do not authorize volume-label mutation or namespace mutation here.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers or validated value types that:
  - validate one external UTF-8 `&str`,
  - materialize UTF-16 units for exFAT consumers,
  - and hand later rows a validated converted-name or converted-label value without exposing generic codec internals.
- They must remain subordinate to `ExfatFs`.

## Allowed Commands

- Read-only shell inspection commands under:
  - `/home/halifuda/asterinas`
  - `/home/halifuda/linux/fs/exfat`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-SYNC-31` designer planning

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current upstream boundaries are still insufficient to specify a clean validated-conversion boundary without reopening fold/hash, namespace mutation, or volume-label control, report the exact missing handshake and stop instead of guessing.
