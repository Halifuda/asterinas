<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-CHARSET-32`
- Title: `ExfatFs` Charset And External-Name Conversion Boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1306-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/00_architect.md`

## Scope

- In scope:
  - State the repeated-call and serialization expectations for the charset conversion boundary.
  - Keep the boundary as a single filesystem-owned service inside `ExfatFs`.
  - Clarify that no dedicated concurrency machinery is needed for this row.
- Out of scope:
  - Background caches, worker threads, lock-free publication, or a second conversion owner.
  - Namespace or volume-label mutation ordering.
  - `EXR-UPCASE-20` fold/hash behavior.

## Serialization Contract

- Shared boundaries involved:
  - Owner-private converted-name state inside `ExfatFs`.
  - Owner-private converted-label state inside `ExfatFs`.
- Rule 1:
  - Each conversion call owns its own validation and UTF-16 materialization; no call should depend on a shared mutable partial result surviving across invocations.
- Rule 2:
  - The converted value becomes visible only after the UTF-16 output is fully validated.
- Rule 3:
  - Repeated calls with the same input should either fail the same way or produce the same validated UTF-16 shape for the same mounted filesystem state.
- Rule 4:
  - The conversion boundary must not hold or expose `EXR-UPCASE-20` state while performing validation.

## Repeated-Call Expectations

- Name conversion:
  - Repeating conversion for the same valid `&str` should yield the same validated converted-name value shape.
  - Repeating conversion for the same invalid input should continue to reject the same invalid shape.
- Label conversion:
  - Repeating conversion for the same valid `&str` should yield the same validated converted-label value shape.
  - Repeating conversion for the same overlong or malformed input should continue to reject it.

## Forbidden Interleavings

- Do not expose a partially filled UTF-16 buffer.
- Do not let a conversion call publish an output that is still being validated.
- Do not let conversion helpers become a shared cache or a second charset policy owner.
- Do not couple validation progress to namespace mutation or label mutation progress.

## Allowed Simplifications

- A fresh conversion path per call is acceptable.
- The existing filesystem-owner serialization boundary is sufficient.
- A simple owner-private wrapper around validated UTF-16 units is acceptable if it remains invisible outside `ExfatFs`.

## Reviewer/Checker Expectations

- Reviewers should confirm that the row stays command-free and owner-local.
- Reviewers should confirm that the only stable output shape is validated UTF-16 text plus length.
- Checkers should confirm repeated-call determinism and rejection stability.
- Checkers should reject any implementation that widens this row into a generic text subsystem.
