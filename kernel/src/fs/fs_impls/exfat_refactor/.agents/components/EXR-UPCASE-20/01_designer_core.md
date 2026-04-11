<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` Upcase Table Ownership And Canonicalization Services
- Status: `Specified`
- Author: designer
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1050-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`

## Scope

- In scope:
  - Store the validated exFAT upcase table as `ExfatFs`-owned runtime state.
  - Provide owner methods that fold UTF-16 name units through that table.
  - Provide the exFAT name-hash service from already-folded name units.
  - Define the validation boundary for table size and checksum before publication.
  - Keep the owner boundary explicit so later lookup and namespace work can consume the canonicalization service without reimplementing it.
- Out of scope:
  - Directory traversal, candidate discovery, mount sequencing, namespace mutation, inode ownership, bitmap policy, and any generic Unicode helper module.
  - A second fallback uppercase table or locale-sensitive text system.
  - Public helper APIs whose only purpose is to expose stored table fields.

## Module Specification

- Dependencies:
  - The raw `Upcase` candidate surfaced by `DirectoryEngine`.
  - The accepted boot and superblock facts already established for the refactor.
  - The Microsoft exFAT upcase-table and name-hash semantics, with the Linux summary used only where the spec leaves room.
- Interfaces provided:
  - `ExfatFs` as the filesystem-wide owner of the validated table.
  - Owner-private `UpcaseTable` state that holds the decoded, validated mapping data.
  - Owner methods for table installation, UTF-16 folding, and exFAT name hashing.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Hidden implementation details:
  - Whether `ExfatFs` stores the table as `Option<UpcaseTable>` or another owner-private state wrapper, provided publication remains atomic and the stored table is immutable after validation.
  - Whether `UpcaseTable` keeps decoded entries, compact spans, or another owner-private representation, provided later calls see the same folding behavior for the lifetime of the mounted filesystem.
  - The exact internal validation helper layout, as long as it stays inside `fs.rs` and does not become a generic text utility module.

## Functional Specification

### Operation

- Name: Install validated upcase table
- Inputs:
  - A raw `Upcase` candidate already discovered by the directory owner.
- Preconditions:
  - The candidate came from the root-directory metadata stream.
  - Table size and checksum are available for validation.
- Actions:
  - Validate the candidate against the exFAT upcase-table rules.
  - Decode the candidate into owner-private `UpcaseTable` state.
  - Publish the table once validation succeeds.
  - Leave the owner unchanged if validation fails.
- Outputs:
  - Success when the table is accepted.
  - An error when the candidate is malformed or inconsistent.
- Postconditions:
  - `ExfatFs` owns one stable validated table for the mount.
  - No later caller needs to rediscover or reparse the table.

### Operation

- Name: Fold UTF-16 name units
- Inputs:
  - A UTF-16 name-unit slice.
- Preconditions:
  - The owner already has a validated upcase table installed.
- Actions:
  - Map each code unit through the installed table.
  - Preserve the same canonicalized result for every later consumer that uses the same mounted filesystem state.
  - Do not consult directory state, mount sequencing, or any locale-specific rule.
- Outputs:
  - Folded UTF-16 name units.
- Postconditions:
  - The folding result is stable for the lifetime of the mounted filesystem.

### Operation

- Name: Compute exFAT name hash
- Inputs:
  - A UTF-16 name-unit slice that has been folded with the installed table.
- Preconditions:
  - The input is already canonicalized through the same `UpcaseTable` instance.
- Actions:
  - Apply the exFAT name-hash algorithm to the folded UTF-16 bytes.
  - Use the same fold result that later lookup and namespace consumers will use.
  - Do not introduce alternate hash policies or generic text normalization.
- Outputs:
  - A `u16` name hash.
- Postconditions:
  - Equal canonical names produce equal hashes.
  - The hash remains stable for the installed table.

## Invariants

- `ExfatFs` owns the only validated upcase table for the mounted filesystem.
- The installed table is immutable after publication.
- Folding and hashing always consume the same table instance.
- No fallback uppercase table is kept alongside the mounted volume table.
- Name hashing is defined over folded UTF-16 bytes, not raw bytes and not UTF-8 text.
- Table validation happens before publication, never after.

## Concurrency Specification

- Shared state:
  - The owner-private upcase-table runtime state inside `ExfatFs`.
- Lock ordering:
  - No new lock hierarchy is introduced here.
- Atomicity requirements:
  - Table validation and publication must be linearized so later callers never observe a partially installed table.
  - Folding and name-hash calls must observe either the old stable state or the new stable state, never an in-between decode.
- Forbidden interleavings:
  - Do not expose a half-decoded table.
  - Do not let folding or hashing race with publication in a way that changes the canonicalization result mid-call.
- Allowed simplifications such as a temporary big lock:
  - The existing filesystem-owner serialization boundary is sufficient for this component.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add owner-private `UpcaseTable` state under `ExfatFs` in `fs.rs`.
  - Implement validated publication of the candidate table before any folding or hashing call can use it.
  - Implement UTF-16 folding through the installed table.
  - Implement exFAT name hashing from folded UTF-16 units.
  - Keep the table logic owner-local and avoid a new helper namespace or mount-sequencing shell.
- Explicit non-goals:
  - No directory scanning.
  - No mount/open sequencing.
  - No namespace mutation.
  - No fallback locale or generic Unicode helper.

### Serial Checker Pass

- Required checker-owned tests:
  - A table-validation regression that accepts a well-formed upcase table and rejects malformed size or checksum data.
  - A folding regression that confirms a mixed-case UTF-16 name folds to the same canonical units on repeated calls.
  - A name-hash regression that confirms case-equivalent names produce the same hash after folding and that distinct folded names do not collapse accidentally.
  - A stability regression that exercises the owner methods without involving directory traversal or mount sequencing.
- Observable properties that must pass before leaving the serial loop:
  - The installed table is the sole source of canonicalization.
  - Folding remains stable and deterministic.
  - Name hashing follows the same folded input that later lookup code will consume.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation required.
- Explicit non-goals:
  - No background refresh.
  - No lock-free canonicalization cache.
  - No async publication protocol.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a single owner-local canonicalization service with one validated table and no extra concurrency machinery.

## Acceptance Notes

- The reviewer should confirm that the design keeps the validated table under `ExfatFs` and does not create a generic text helper module.
- The reviewer should reject any attempt to fold directory discovery or mount sequencing into this component.
- The reviewer should verify that the name-hash service is explicitly defined over folded UTF-16 bytes.
- Any creator split should stay serialized on `fs.rs`, because both table ownership and canonicalization services land in the same owner file.
