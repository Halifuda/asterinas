<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: Linux Prior Enrichment Wave

**Date / Time:** April 17, 2026, 15:05 CST
**Status:** Handed Over

## 1. Global State Pointer
*Always read `SYSTEM_BLUEPRINT.md` for the overarching project state. This section only notes immediate shifts made during this thread.*
- **Current Active Wave / Pass:** Linux Prior Enrichment
- **Blueprint Updates Made:** No. This session stayed entirely inside the prior layer and did not advance `SYSTEM_BLUEPRINT.md` or any Architect/Designer/Creator pass.

## 2. Pass Slicing Decisions
*Record the non-default pass boundaries chosen by the main agent. This is mandatory whenever a meso-component is split into multiple Creator/Checker passes.*
- None. No meso component was sliced in this session.

## 3. Thread Activity Log (The Active Wave)
*What did this specific main-agent session actually do? Keep it concise. Focus on scheduling and dispatching.*
- **Dispatches Sent:**
  - None. The work was a local main-agent verification and prior-writing pass against `~/linux/fs/exfat`.
- **Acceptance Outcomes:**
  - `Prior Layer - Main Agent - linux-exFAT pending cleanup` -> Accepted. Resolved the previously open Linux-side uncertainty on `fallocate`, `statfs`, and lookup `i_pos` behavior in `priors/linux-exFAT-implementation-summary.md`.
  - `Prior Layer - Main Agent - linux-exFAT name-cache enrichment` -> Accepted. Added dentry revalidation, case-folded hash/compare, alias reuse, and case-only rename limitation semantics.
  - `Prior Layer - Main Agent - linux-exFAT runtime I/O enrichment` -> Accepted. Added `valid_size` zero-fill, direct-I/O alignment constraints, mmap/write-fault semantics, forced-shutdown fast-fails, and stronger `fsync` semantics.
  - `Prior Layer - Main Agent - linux-exFAT maintenance ABI enrichment` -> Accepted. Added `FITRIM`, shutdown, volume-label, DOS-attribute ioctl, and rename-flag boundary semantics.
- **Escalations / Deadlocks:**
  - None.

## 4. Explicit Agent-Level Decisions
*Record non-automated choices made by the main agent during this session. (e.g., clearing stale locks, skipping final reviewer, reopening a Creator Pass after an integration failure).*
- Chose **not** to start `ASTERINAS_MICRO_FEATURE_INVENTORY.md` in this wave, matching the user's explicit budget decision.
- Treated `~/linux/fs/exfat` as the authoritative tie-breaker for Linux VFS behavior instead of relying on prior prose assumptions.
- Consolidated the Linux prior around three now-closed certainty points:
  - `fallocate` is definitively `-EOPNOTSUPP` for exFAT because `.fallocate` is absent and VFS provides no filesystem fallback.
  - `statfs` uses cached `sbi->used_clusters`, seeded by mount-time bitmap scan and then maintained incrementally by alloc/free paths.
  - `i_pos` is a private inode-cache key built from `(dir cluster, dentry index)`, reused by lookup/create/mkdir/readdir and rehashed on rename.
- Extended the Linux prior into additional high-value surfaces:
  - **Name-cache / dentry coherence**: negative dentry invalidation, `iversion`-based revalidation, case-folded hashing/comparison, alias reuse, and positive case-only rename limitations.
  - **Runtime file I/O**: `valid_size` gap zero-fill, direct-I/O alignment rejection, generic read/write/splice delegation, mmap page-fault extension of `valid_size`, and forced-shutdown `-EIO` fast-fails.
  - **Administrative ABI**: DOS-attribute ioctls, `FITRIM`, forced shutdown, volume-label get/set, and the rule that forced shutdown disables online discard.
  - **Operation boundaries**: `rename` only honoring `RENAME_NOREPLACE` and rejecting all other rename flags.
- Concluded that the Linux prior is still **not** exhausted, but the remaining profitable work is now narrower and more selective than before. The strongest remaining candidates are:
  - Mount/remount option semantics and how they reshape runtime behavior (`discard`, `keep_last_dots`, UTF-8 vs NLS paths, zero-size-dir handling).
  - Attribute / timestamp update ordering around create/unlink/rename/mkdir and how much of that should become explicit Designer-facing prior.
  - More exact block-mapping / page-cache interaction details (`bmap`, `get_block`, `truncate_pagecache`, writeback boundaries) if we later need tighter Designer constraints.

## 5. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*
1. If continuing Linux-prior-only work, read `~/linux/fs/exfat/super.c` option parsing / remount paths and `~/linux/fs/exfat/namei.c` mount-sensitive name handling to document mount-option-dependent semantics (`discard`, `keep_last_dots`, UTF-8 vs NLS, zero-size-dir) in `priors/linux-exFAT-implementation-summary.md`.
2. If Linux prior depth is judged sufficient, stop expanding priors and ask the user whether to reopen the Inventory scope or redirect effort into another upstream artifact. Do **not** start `ASTERINAS_MICRO_FEATURE_INVENTORY.md` unless the user explicitly reopens that scope.
