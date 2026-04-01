<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Final Report

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Role: checker
- Date: 2026-04-01
- Authorizing main agent: main-agent

## Baseline

- Serial checker evidence: `11_checker_serial.md`
- Reviewer report: `30_reviewer_report.md`

The reviewer reported no code changes, so this pass reran the same focused identity and lookup ktest surface in serial order.

## Runtime Evidence

### KVM preflight

Command:

```bash
docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'
```

Observed:

- `no-kvm`

Interpretation:

- QEMU fell back to TCG in this environment.

## Verification Run

All commands were run sequentially and filtered to the component-specific ktest names.

1. Command:

```bash
docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_key_preserves_packed_location_layout'
```

Observed:

- passed successfully under TCG.

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

## Outcome

Acceptance-ready for EXR-INOKEY-05A.

The required focused identity and exact-match lookup ktests passed in sequence, and the runtime environment is confirmed to be TCG-backed because `/dev/kvm` was not available.
