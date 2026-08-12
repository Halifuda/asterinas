<!-- SPDX-License-Identifier: MPL-2.0 -->

# Asterinas Code Quality Priors (Strict Rulebook)

This document is the absolute, objective checklist of coding standards for the Asterinas OS, derived directly from the repository-root `AGENTS.md` and the `book/src/to-contribute/coding-guidelines/`.

Under the Top-Down Strict Protocol, this file serves as the strict operational boundary for Creators (when writing code) and Reviewers (when auditing). It contains NO agent-workflow meta-instructions or role-profile slicing.

For the overlayfs refactor, the validation-method rule overrides any generic
testing guidance below: this refactor is xfstests-only and must not create,
modify, or grow any ktest-based validation surface. The testing notes below do
not authorize `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test modules,
`test_support/`, or memory-disk fixture changes in this workspace.

## CORE CODING STANDARDS & RUST HYGIENE

### 1. Unsafe Code
- `kernel/` is entirely Safe Rust. **Never** introduce `unsafe` blocks, `unsafe fn`, or `unsafe trait` inside `kernel/core/src/fs/`.
- Every crate under `kernel/` must maintain `#![deny(unsafe_code)]`.
- `ostd/` is the only crate permitted to use `unsafe`.

### 2. Error Handling & Panics
- **Never use `.unwrap()` or `.expect()`** on fallible operations (Options/Results) where failure is possible in production code.
- Propagate errors idiomatically using the `?` operator.
- Dismiss error/edge cases early in the function (early returns) to keep the "happy path" unindented.

### 3. Concurrency & Locks
- Never perform I/O (especially Block I/O or `Bio` calls) or blocking operations while holding a spinlock.
- Establish and document lock order hierarchically to prevent deadlocks.
- Critical sections (check + action) must be atomic under the same lock; avoid TOCTOU (Time-of-Check to Time-of-Use) internal races.
- Avoid casual use of atomics. Use locks for correlated fields.

### 4. Visibility & Imports
- Default to the narrowest possible visibility: `pub(self)` -> `pub(super)` -> `pub(crate)`. Use `pub` only when crossing major boundaries.
- Import free functions and statics via their parent module, not directly. (e.g., `use ostd::irq; irq::disable_local();` instead of `use ostd::irq::disable_local; disable_local();`).
- All crates must use `[workspace.dependencies]` via `workspace = true`.

### 5. Types, Functions & Arithmetic
- Functions must be small, focused, and represent one concept. Nesting should be kept to a maximum of 3 levels.
- Avoid boolean arguments; split into two functions or use an explicit `enum`.
- Use types to enforce invariants. Prefer `enum` over trait objects for closed sets. Encapsulate fields behind getters.
- Always use **checked or saturating arithmetic** to prevent panics on overflow, or explicit `wrapping_*` methods if wraparound is intended.

### 6. Logging
- Only use OSTD logging macros (`debug!`, `info!`, `warn!`, `error!`, `crit!`, `emerg!`).
- Import via `use ostd::prelude::*`.
- **Never** use `println!`, the third-party `log` crate, or manual serial prints in production code.

### 7. Naming Conventions
- Types/Structs: CamelCase with title-cased acronyms (e.g., `IoMemoryArea`).
- Functions/Vars: `snake_case`. Be descriptive. No single-letter names or ambiguous abbreviations.
- Closure variables: End with `_fn`.
- Boolean functions/vars: Must use prefixes like `is_`, `has_`, `can_`, `should_`, `needs_`.
- Unit encoding: Encode physical units into variable names if the data type itself does not convey the unit (e.g., use `timeout_ns` or `size_pages` instead of just `timeout` or `size`).

### 8. Attributes & Documentation
- Doc comments: First line uses third-person singular present ("Returns", "Creates"). End sentence comments with punctuation.
- Wrap identifiers in backticks. Explain *why*, not *what*.
- Prefer functions over macros.
- Prefer `#[expect(...)]` over `#[allow(...)]` for suppressing lints natively.
- Sort non-derive outer attributes alphabetically. Place `#[derive(...)]` last, and sort traits inside it alphabetically.

### 9. Testing & Review
- Test user-visible behavior through public APIs, not internals.
- Clean up resources after every test.
- For this overlayfs refactor, do not add or modify kernel-mode test-only
  helpers; validate through the packeted upstream xfstests lane instead of
  ktest modules.
