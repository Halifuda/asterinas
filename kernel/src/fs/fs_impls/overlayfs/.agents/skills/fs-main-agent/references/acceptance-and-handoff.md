# Acceptance And Handoff

Use this note when deciding whether to accept artifacts or when updating the active main-agent note.

## Structural acceptance

- Validate against the repo template, not against your preferred prose style.
- Missing required sections are a protocol violation even if the artifact sounds logically plausible.
- Keep artifact families in the required locations under `.agents/components/<component-id>/`.
- Reject Creator, Checker, or Reviewer artifacts that omit the parent meso-component or covered micro-features required by the pass templates.

## Checker evidence

- Green exit status is not enough for filtered or partial upstream-suite runs.
- Checker evidence must include the exact command, proof that the intended upstream tests executed, and preserved guest log / suite-result evaluation when QEMU or NixOS xfstests was involved.
- New filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/` is a rejection condition for new work.
- The main agent should pass Checker repair batches through to Creator work without reinterpretation.
- Keep Creator-synced Checker passes separate from meso-level integration passes when evaluating coverage and acceptance.

## Retry escalation

- If the Creator/Checker loop fails 5 times without a passing upstream-approved validation receipt, stop the loop.
- Escalate the impasse back to Designer or Architect with the accumulated failure history.

## Continuous handoff

- The handoff is an actively maintained wave log.
- Record dispatches, pass-slicing decisions, acceptance or rejection decisions, and escalations during the wave.
- End each session with explicit next-main-agent actions for the next context window.
