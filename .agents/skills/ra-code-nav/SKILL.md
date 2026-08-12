---
name: ra-code-nav
description: Read-only Rust code navigation for the Asterinas workspace using rust-analyzer's LSIF index. Use when an agent needs symbol-aware lookup — workspace symbol search, goto-definition, find-references, hover, document symbols, or implementation lookup — without spawning a long-lived LSP server. Queries are shell + jq against a pre-generated LSIF JSONL index. Works inside the `codex-asterinas-dev` container.
---

# ra-code-nav

Read-only Rust code navigation for the Asterinas workspace using
**rust-analyzer's LSIF index** — a one-shot JSONL dump that can be queried with
`jq` without keeping an LSP server alive.

## When to use

Use this skill whenever an agent needs **symbol-aware** Rust navigation:
- "where is `OverlayFs` defined?"
- "who references `ovl_inode->lock`?"
- "show me the hover/doc for this symbol"
- "list all symbols in this file"
- "find all impls of trait `Inode`"

Do NOT use this skill for:
- plain text grep (use `grep`/`rg` directly)
- editing code (this skill is strictly read-only)
- non-Rust files

## Why LSIF, not LSP

The previous workflow used a Python LSP client (`ra_code_nav.py`) that was
fragile: long-lived stdio JSON-RPC, timing-sensitive waits for project load,
and frequent desync. LSIF is a **one-shot offline index**: `rust-analyzer lsif`
dumps the entire project's symbol/definition/reference/hover data to a JSONL
file in ~30s, after which every query is a stateless `jq` filter. No server, no
timing, no Python.

## Prerequisites

- The `codex-asterinas-dev` Docker container must be running.
- `rust-analyzer` and `jq` are installed in the container (verified).
- The Asterinas source is mounted at `/root/asterinas` in the container.

## Index generation (one-shot, ~30s)

Generate the LSIF index once per working session (or after major code changes):

```bash
docker exec codex-asterinas-dev bash -c \
  "cd /root/asterinas && rust-analyzer lsif . > /tmp/asterinas.lsif 2>/tmp/lsif.log"
```

The index is ~155MB JSONL (one JSON object per line). Regenerate when the code
changes substantially; for small edits the stale index is usually good enough
for symbol lookup.

## Rebuild strategy

LSIF generation is a **full rebuild, not incremental** (verified: no
`--incremental` flag, no on-disk salsa cache, ~30s regardless of change size,
deterministic output for identical source). Rebuild policy:

- **Per wave**: regenerate once at the start of each implementation wave, or
  when the Architect/Designer needs to query symbols introduced since the last
  rebuild. 30s is a fixed, acceptable cost.
- **Within a wave**: do NOT rebuild for every small edit. The stale index is
  fine for symbol lookup — definitions and references rarely move far. Only
  rebuild if a query returns nothing for a symbol you know was just added.
- **Creator passes**: Creators should NOT rebuild during a pass. Their packet
  scope is small enough to read files directly. LSIF is for Architect/Designer
  cross-cutting navigation.
- **Pre-split for speed**: if a wave does many queries, pre-split the index
  once (see Performance notes below) to cut per-query latency from ~2s to ~0.1s.

Rationale for LSIF over a long-lived LSP server: agent workflows query symbols
a handful of times per wave, not continuously like an IDE. The 30s rebuild +
0.1-2s/query is a better tradeoff than managing a stateful bidirectional
JSON-RPC server with no Python client available.

## Query recipes

All queries run via `docker exec codex-asterinas-dev bash -c "..."` with `jq`
filtering the LSIF file at `/tmp/asterinas.lsif`. The recipes below are
copy-paste ready. Replace `OverlayFs` with the target symbol.

### 1. Workspace symbol search (by name)

```bash
docker exec codex-asterinas-dev bash -c '
  grep "\"label\":\"moniker\"" /tmp/asterinas.lsif \
  | jq -c "select(.identifier | test(\"OverlayFs\"))" '
```

Returns moniker vertices with full qualified path (e.g.
`aster_kernel::fs::fs_impls::overlayfs::fs::OverlayFs`).

### 2. Goto definition (by moniker identifier)

```bash
docker exec codex-asterinas-dev bash -c '
  IDENT="aster_kernel::fs::fs_impls::overlayfs::fs::OverlayFs"
  # 1. moniker -> resultSet
  RS=$(grep "\"label\":\"moniker\"" /tmp/asterinas.lsif \
    | jq -r "select(.identifier==\"$IDENT\") | .id")
  # 2. resultSet -> definitionResult
  DR=$(grep "\"type\":\"edge\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$RS and .label==\"textDocument/definition\") | .inV")
  # 3. definitionResult -> item edge -> range IDs
  RIDS=$(grep "\"type\":\"edge\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$DR and .label==\"item\") | .inVs[]"")
  # 4. range IDs -> positions + document
  for rid in $RIDS; do
    grep "\"label\":\"range\"" /tmp/asterinas.lsif \
    | jq -c "select(.id==$rid)"
  done
'
```

### 3. Find references (by moniker identifier)

```bash
docker exec codex-asterinas-dev bash -c '
  IDENT="aster_kernel::fs::fs_impls::overlayfs::fs::OverlayFs"
  RS=$(grep "\"label\":\"moniker\"" /tmp/asterinas.lsif \
    | jq -r "select(.identifier==\"$IDENT\") | .id")
  RR=$(grep "\"type\":\"edge\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$RS and .label==\"textDocument/references\") | .inV")
  # referenceResult has item edges with property=references
  RIDS=$(grep "\"type\":\"edge\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$RR and .label==\"item\" and .property==\"references\") | .inVs[]")
  for rid in $RIDS; do
    grep "\"label\":\"range\"" /tmp/asterinas.lsif \
    | jq -c "select(.id==$rid)"
  done
'
```

### 4. Hover (by moniker identifier)

```bash
docker exec codex-asterinas-dev bash -c '
  IDENT="aster_kernel::fs::fs_impls::overlayfs::fs::OverlayFs"
  RS=$(grep "\"label\":\"moniker\"" /tmp/asterinas.lsif \
    | jq -r "select(.identifier==\"$IDENT\") | .id")
  HR=$(grep "\"type\":\"edge\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$RS and .label==\"textDocument/hover\") | .inV")
  grep "\"label\":\"hoverResult\"" /tmp/asterinas.lsif \
  | jq -c "select(.id==$HR)"
'
```

### 5. Document symbols (by file path)

```bash
docker exec codex-asterinas-dev bash -c '
  FILE="file:///root/asterinas/kernel/core/src/fs/fs_impls/overlayfs/fs.rs"
  # find document vertex
  DOC=$(grep "\"label\":\"document\"" /tmp/asterinas.lsif \
    | jq -r "select(.uri==\"$FILE\") | .id")
  # find contains edge -> range IDs
  RIDS=$(grep "\"label\":\"contains\"" /tmp/asterinas.lsif \
    | jq -r "select(.outV==$DOC) | .inVs[]")
  for rid in $RIDS; do
    grep "\"label\":\"range\"" /tmp/asterinas.lsif \
    | jq -c "select(.id==$rid)"
  done
'
```

### 6. Implementation lookup

LSIF does not have a direct `implementation` result type. To find impls of a
trait/struct, use **workspace symbol search** with the impl path pattern, or
fall back to `rust-analyzer symbols` on the specific file via stdin:

```bash
docker exec codex-asterinas-dev bash -c '
  cat /root/asterinas/kernel/core/src/fs/fs_impls/overlayfs/fs.rs \
  | rust-analyzer symbols \
  | jq -c "select(.kind==\"SymbolKind(Impl)\")" '
```

## LSIF schema cheat sheet

```
Vertices:  metaData, document, range, resultSet, hoverResult,
            definitionResult, referenceResult, moniker, packageInformation
Edges:      next (range->resultSet), contains (document->range[]),
            textDocument/hover (resultSet->hoverResult),
            textDocument/definition (resultSet->definitionResult),
            textDocument/references (resultSet->referenceResult),
            item (result->range[], with document + optional property:
                  "definitions" | "references"),
            moniker (resultSet->moniker)
```

Key traversal: `moniker.identifier` → `moniker edge outV` = resultSet →
`textDocument/*` edges → result vertex → `item` edge `inVs` = range IDs →
range vertex has `start`/`end` `{line, character}` (0-indexed, utf-16).

## Performance notes

- Each query greps the full 155MB file. On a warm filesystem cache this is
  ~1-3s per grep. For repeated queries, consider pre-splitting the index:
  ```bash
  docker exec codex-asterinas-dev bash -c "
    grep '\"label\":\"moniker\"' /tmp/asterinas.lsif > /tmp/ra-monikers.jsonl
    grep '\"label\":\"range\"' /tmp/asterinas.lsif > /tmp/ra-ranges.jsonl
    grep '\"type\":\"edge\"' /tmp/asterinas.lsif > /tmp/ra-edges.jsonl
  "
  ```
  Then query the smaller files for faster lookups.

## Limitations

- LSIF is a snapshot; it does not reflect edits after generation. Regenerate
  after significant code changes.
- `implementation` lookup is not directly supported by LSIF; use `symbols` or
  workspace symbol search as a fallback.
- Conditional compilation (`#[cfg(target_arch = "...")]`) means some symbols
  only exist for one arch. The index reflects the default target (x86_64).
- The index includes all workspace crates (kernel, ostd, osdk, tests, deps).
  Filter by moniker prefix (e.g. `aster_kernel::`) to scope results.

## Container quick check

```bash
docker exec codex-asterinas-dev bash -c \
  "test -f /tmp/asterinas.lsif && echo 'index ready' || echo 'index missing — run generation step'"
```
