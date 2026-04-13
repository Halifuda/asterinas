<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

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
  - Define the smallest `ExfatFs`-owned conversion surface that accepts Asterinas `&str` names or volume-label strings and produces validated UTF-16 values for later exFAT consumers.
  - Repair existing read-side name consumers so `ExfatInode::lookup` and `readdir_at` stop performing local UTF-16 conversion or decode policy in `inode.rs`.
  - Make the validated output shape explicit so later rows do not guess whether the boundary returns raw text, hashes, or mutation-ready namespace objects.
  - Keep `EXR-UPCASE-20` as the only owner of fold and hash behavior over UTF-16 units.
  - Define the downstream handoff to `EXR-NAMESPACE-29` and `EXR-VOLLABEL-35` as consumed validated values, not as ad hoc string parsing.
  - Define the read-side handoff back to VFS-visible `String` values so accepted directory-ops code paths consume the same owner instead of using ad hoc `String::from_utf16()` helpers.
  - State repeated-call expectations for the conversion service without creating a generic Unicode helper module.
- Out of scope:
  - Namespace mutation, volume-label mutation, directory traversal, mount sequencing, and allocation policy.
  - UTF-16 fold/hash logic, canonical name comparison, or any second charset policy.
  - Linux-style byte-string or locale-sensitive NLS as a second stable contract.

## Module Specification

- Dependencies:
  - The Asterinas VFS name contract, which presents user-visible names as Rust `&str`.
  - `EXR-UPCASE-20` for fold/hash behavior on already-converted UTF-16 units.
  - The accepted `ExfatFs` owner boundary and its filesystem-wide state.
  - The exFAT name and volume-label limits recorded in the Microsoft spec and the authorized Linux references.
- Interfaces provided:
  - Owner-private `ConvertedName` and `ConvertedLabel` value types under `ExfatFs`.
  - `ExfatFs` methods that convert `&str` input into validated UTF-16 values.
  - `ExfatFs` handoff helpers that return the validated converted value to later namespace or volume-label callers.
  - `ExfatFs` helper(s) that decode validated on-disk UTF-16 name units into VFS-visible `String` output for read-side callers.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` as the migration point for legacy `lookup` and `readdir_at` consumers
- Hidden implementation details:
  - Whether the validated value is stored as one generic UTF-16 text wrapper with name and label variants, or as two owner-private wrappers that share the same internal representation.
  - Whether the wrappers own a small fixed UTF-16 buffer or another owner-private representation, provided callers only observe validated length plus UTF-16 units.
  - Whether conversion helpers are factored into a few local steps inside `fs.rs`, provided they remain owner-local and do not become a generic text utility module.
  - Whether read-side visible-name decode is exposed as one dedicated owner method or a small paired helper family, provided `inode.rs` stops choosing conversion policy locally.
  - Whether low-level on-disk constructors such as `ExfatDentrySet::from_trusted_metadata(..., raw_name_units, ...)` remain available as trusted leaf seams, provided name-bearing business paths stop calling them with ad hoc UTF-16 built outside the charset owner.

## Functional Specification

### Operation

- Name: Convert external name text
- Inputs:
  - A VFS-visible `&str` name from Asterinas.
- Preconditions:
  - The caller is providing UTF-8 text, not a Linux byte-string contract.
  - The candidate name is intended for exFAT namespace use.
- Actions:
  - Validate the input as a bounded exFAT external name.
  - Convert the text into UTF-16 units.
  - Reject invalid shape early instead of deferring charset policy to namespace mutation.
  - Publish the converted value only after the UTF-16 result is fully validated.
- Outputs:
  - An owner-private validated converted-name value.
- Postconditions:
  - Later namespace work consumes validated UTF-16 units, not raw text.
  - No fold or hash result is attached by this row.

### Operation

- Name: Decode visible directory name text
- Inputs:
  - Validated on-disk UTF-16 name units from a directory record.
- Preconditions:
  - The caller is projecting an already-validated exFAT file record into a VFS-visible name.
- Actions:
  - Decode UTF-16 units into a visible Rust `String`.
  - Reject malformed UTF-16 instead of leaving decode policy to `inode.rs`.
  - Publish the visible string only after decoding succeeds completely.
- Outputs:
  - A VFS-visible `String` for read-side directory consumers.
- Postconditions:
  - `readdir_at` and similar read-side consumers do not call `String::from_utf16()` directly.
  - Read-side visible-name projection crosses the same filesystem-owned charset boundary.

### Operation

- Name: Convert external volume-label text
- Inputs:
  - A VFS-visible `&str` label string from Asterinas.
- Preconditions:
  - The caller is providing UTF-8 text for volume-label control.
- Actions:
  - Validate the label shape against the exFAT volume-label limit.
  - Convert the text into UTF-16 units.
  - Publish the converted value only after the UTF-16 result is fully validated.
- Outputs:
  - An owner-private validated converted-label value.
- Postconditions:
  - Later volume-label work consumes validated UTF-16 units, not raw text.
  - No fold or hash result is attached by this row.

### Operation

- Name: Reuse converted text
- Inputs:
  - A repeated `&str` name or label string.
- Preconditions:
  - The caller is invoking the same conversion service again on the same mounted filesystem state.
- Actions:
  - Re-run validation and conversion deterministically.
  - Return the same validated UTF-16 value shape for the same input and filesystem state.
  - Do not allocate a second generic helper layer or a text cache that escapes `ExfatFs`.
- Outputs:
  - The same validated converted value shape as the first successful call.
- Postconditions:
  - The conversion boundary is stable and repeatable.

## Invariants

- `ExfatFs` owns the only stable charset-conversion boundary for VFS-visible exFAT names and labels.
- The validated output is UTF-16 text plus length, not a hash and not a canonicalized lookup key.
- `EXR-UPCASE-20` remains the only owner of fold and hash behavior over UTF-16 units.
- The row does not introduce a second NLS or locale-sensitive text contract.
- The row does not expose a generic Unicode helper module.
- Later consumers must not reparse raw `&str` if they already received a validated converted value from this row.
- Legacy inode-side consumers must not keep local `encode_utf16()` or `String::from_utf16()` policy once this row lands.
- Low-level trusted constructors may still accept raw UTF-16 units, but business-facing callers must reach them only through validated converted-name or converted-label handoffs from `ExfatFs`.

## Concurrency Specification

- Shared state:
  - The owner-private conversion state inside `ExfatFs`.
  - The owner-private visible-name decode state inside `ExfatFs`.
- Lock ordering:
  - No new lock hierarchy is introduced here.
- Atomicity requirements:
  - Validation and publication of a converted value must be linearized so later callers never observe a partially converted result.
  - Repeated callers must observe either a rejected input or a fully validated UTF-16 value.
- Forbidden interleavings:
  - Do not expose a partially populated UTF-16 buffer.
  - Do not let charset conversion race with publication in a way that changes the validated shape mid-call.
- Allowed simplifications such as a temporary big lock:
  - The existing filesystem-owner serialization boundary is sufficient for this component.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add owner-private validated converted-name and converted-label value types under `ExfatFs` in `fs.rs`.
  - Implement UTF-8 `&str` validation and UTF-16 conversion for names and labels.
  - Add the owner method or owner-private helper that decodes validated UTF-16 record-name units back to VFS-visible `String`.
  - Migrate the existing `lookup` and `readdir_at` code paths in `inode.rs` so they consume the `ExfatFs` charset owner instead of performing local conversion or decode policy.
  - Keep the validation boundary owner-local and avoid a generic Unicode helper module.
  - Keep the conversion service separate from fold/hash and separate from namespace or volume-label mutation.
- Explicit non-goals:
  - No namespace mutation.
  - No volume-label mutation.
  - No `EXR-UPCASE-20` canonicalization behavior.
  - No Linux-style byte-string or locale policy layer.

### Serial Checker Pass

- Required checker-owned tests:
  - A name-conversion regression that accepts a valid UTF-8 exFAT name and returns a validated UTF-16 value.
  - A label-conversion regression that accepts a valid UTF-8 volume-label string and returns a validated UTF-16 value.
  - A read-side visible-name regression that decodes validated UTF-16 units through `ExfatFs` and rejects malformed UTF-16 without leaving decode policy in `inode.rs`.
  - A rejection regression that fails malformed or overlong input without publishing a partial converted value.
  - A repeated-call regression that confirms the same input returns the same validated output shape for the same mounted filesystem state.
- Observable properties that must pass before leaving the serial loop:
  - The validated output is UTF-16 text plus length only.
  - The conversion boundary remains filesystem-owned and owner-local.
  - Existing read-side consumers cross the same owner boundary for visible-name decode.
  - No fold/hash behavior is introduced by this row.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation required.
- Explicit non-goals:
  - No background refresh.
  - No lock-free conversion cache.
  - No async publication protocol.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a single owner-local conversion service with validated UTF-16 output and no extra concurrency machinery.

## Acceptance Notes

- The reviewer should confirm that the validated output shape is UTF-16 units plus length, with no hash attached.
- The reviewer should confirm that `EXR-NAMESPACE-29` consumes the validated converted-name value and then hands its UTF-16 units to `EXR-UPCASE-20` for fold/hash work.
- The reviewer should confirm that `EXR-VOLLABEL-35` consumes the validated converted-label value from this same boundary and does not open a second charset policy.
- The reviewer should confirm that accepted read-side consumers such as `lookup` and `readdir_at` no longer keep local `encode_utf16()` or `String::from_utf16()` policy in `inode.rs`.
- The reviewer should reject any attempt to turn this row into a generic Unicode helper or Linux-style NLS layer.
