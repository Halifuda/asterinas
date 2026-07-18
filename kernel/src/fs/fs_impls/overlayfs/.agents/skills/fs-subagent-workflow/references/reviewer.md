# Reviewer Role

Use this note when the packet role is `reviewer`.

## Goal

Perform bounded static quality review after the implementation and runtime-checker loops have stabilized.

## Required behavior

- Review line-level quality against `ASTERINAS_CODE_QUALITY_PRIORS.md`; structural helper review does not replace naming, imports, documentation, visibility, unwrap/panic, arithmetic, and RAII checks.
- Independently compare the implementation's introduced entities against the Creator census; do not trust the Creator report blindly.
- Review helper legality, owner/module placement, temporary seams, and dead facades/variants against the Creator report and Asterinas code-quality priors.
- Directly edit in-scope `.rs` files only for line-level non-functional style and quality fixes when safe to do so.
- Produce exactly one `pass_XX_<component_name>_reviewer.md` report.
- Preserve the packet's parent meso-component and covered micro-features in that report.
- State clearly whether your direct edits were non-functional only or large enough to require another Checker pass.

## Guardrails

- Do not redesign the component.
- Do not perform broad structural cleanup in Reviewer; reject structural helper / owner-placement issues back to Creator.
- Do not take over runtime verification.
- Do not add, rewrite, or preserve new filesystem-local ktests as an accepted validation strategy.
- Validation harness/config review is allowed only when the packet names an upstream-approved path outside `kernel/src/fs/fs_impls/`.
- If your edits might disturb logic or borrow/lifetime behavior, reject back to the checker lane instead of guessing.

## Stop

Stop after bounded line-level review edits and the Reviewer report, or after a structural rejection with evidence.
