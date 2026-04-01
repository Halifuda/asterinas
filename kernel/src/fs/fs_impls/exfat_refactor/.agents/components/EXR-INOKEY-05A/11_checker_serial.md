<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Log

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Role: checker
- Date: 2026-04-01
- Authorizing main agent: main-agent

## Scope Checked

Validated the serial creator pass against the designer obligations for:

- stable packed inode-key construction,
- explicit root special-case construction,
- packed-offset overflow rejection,
- exact-match read-only opened-inode lookup.

## Runtime Evidence

### KVM preflight

Command:

```bash
docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'
```

Observed:

- `no-kvm`

Interpretation:

- QEMU runs in this environment used TCG fallback, not KVM.

### Filtered ktest sequence

1. Command:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_key_preserves_packed_location_layout'
```

Observed:

- initial run hit a checker-owned compile error because the local `#[ktest]` attribute macro was not imported in the new test modules,
- after adding `use ostd::prelude::ktest;` to the local test blocks, the rerun completed successfully under TCG.

2. Command:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_inode_key_is_reserved'
```

Observed:

- passed successfully under TCG.

3. Command:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_key_rejects_offset_overflow'
```

Observed:

- passed successfully under TCG.

4. Command:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test opened_inode_lookup_is_exact_match'
```

Observed:

- passed successfully under TCG.

## Checker-Owned Test Coverage Added

- [`inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs): `inode_key_preserves_packed_location_layout`, `root_inode_key_is_reserved`, `inode_key_rejects_offset_overflow`.
- [`fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs): `opened_inode_lookup_is_exact_match`.

## Outcome

The required checker-owned coverage is now present and the four mandated filtered ktests pass in sequence in this environment.
