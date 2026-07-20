<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff — 2026-07-20

## Session Summary

Completed step 0 (agent workflow bootstrap) and step 1 (priors layer staging)
for the overlayfs refactor workspace. The branch `codex/overlayfs-refactor` is
based on `upstream/main` (2b34b1051) and contains 4 commits:

- `b2d0df22a` Bootstrap overlayfs refactor multi-agent workspace (step 0)
- `f03ef716b` Populate overlayfs priors layer (step 1)
- `45362e19f` Add ra-code-nav skill (LSIF + jq), retire ra_code_nav.py
- `ff1c00a2f` Map xfstests overlay tests to micro-features

## State of Priors

All four overlayfs-specific priors are populated under `.agents/priors/`:

- `REFERENCE_IMPLEMENTATION_SUMMARY.md` — Linux `~/linux/fs/overlayfs/` reference (496 lines, 12 sections)
- `FILESYSTEM_SPEC_SUMMARY.md` — authoritative spec from overlayfs.rst (438 lines, 22 sections)
- `FILESYSTEM_SPEC_INDEX.md` — quick-lookup index (121 lines, 5 tables)
- `MICRO_FEATURE_INVENTORY.md` — 81 micro-features in 4 tiers (P0=18, P1=37, P2=17, P3=9) plus xfstests test mapping

## Critical Architecture Discoveries

1. **Asterinas VFS does NOT hold a parent-directory lock.** Linux overlayfs
   relies on VFS holding `i_rwsem`; Asterinas does not. The overlay MUST
   introduce a new `DIR` lock domain (Mutex) as the outermost overlay-owned
   lock for directory consistency. This is recorded in
   `MICRO_FEATURE_INVENTORY.md` "Asterinas-Specific Architect Notes" §1.

2. **Only `ostd::sync::Mutex` is a safe sleep lock.** `RwLock` is spin-based
   (`PreemptDisabled`). Any critical section crossing BIO must use `Mutex`.
   `INODE`, `CUL`, `UPPER`, and the new `DIR` domain all cross BIO. Recorded
   in same §2.

3. **No reentrant locks exist in Asterinas.** Architect must identify all
   reentrant paths. Recorded in same §3.

4. **`page_cache()` forwarding may trigger copy-up** (P1-37). VFS mmap path
   that holds a lock while invoking `page_cache()` must not violate the
   hierarchy. Recorded in same §4.

## Rust-Analyzer / Code Navigation

- `rust-analyzer` 1.96.0-nightly is fully operational inside the
  `codex-asterinas-dev` Docker container.
- LSIF index generation: `docker exec codex-asterinas-dev bash -c "cd /root/asterinas && rust-analyzer lsif . > /tmp/asterinas.lsif"` (30s, 155MB, full deterministic rebuild, no incrementality).
- All 6 LSP query types verified: workspace/symbol, goto-definition, find-references, hover, documentSymbol, implementation.
- New skill: `ra-code-nav` (`.agents/skills/ra-code-nav/SKILL.md`) — shell + jq recipes for LSIF queries, rebuild strategy.
- Old `ra_code_nav.py` references replaced across 7 protocol/skill files.
- LSIF is per-wave rebuild, not per-edit. Rebuild cost: ~30s fixed.

## PR #3298 (multi-fs xfstests) — Cherry-Pick Issues

PR #3298 (Fischer0522, "Support multi file systems for `xfstests`") provides the
generic framework to run xfstests against multiple filesystems, replacing
hard-coded ext2 support with per-fs configuration directories
(`ext2/`, `tmpfs/`, `template/`). It has 2 commits on top of a 3-month-old
base (42d38f9af, 103 commits behind `upstream/main`).

**Cherry-pick attempt of first commit** (`f4459b85a`, "Support multi file
systems for `xfstests`") results in **4 merge conflicts**:

| File                                                                   | Conflict          | Root Cause                                        |
| :--------------------------------------------------------------------- | :---------------- | :------------------------------------------------ |
| `Makefile`                                                             | 1 conflict marker | `XFSTESTS_FS_TYPE` variables vs main's newer vars |
| `test/initramfs/Makefile`                                              | 1 conflict marker | Build targets diverged in intervening 103 commits |
| `test/initramfs/src/conformance/xfstests/run_xfstests.sh`              | 1 conflict marker | Runner script modified by subsequent PRs on main  |
| `test/initramfs/src/conformance/xfstests/tmpfs/config/xfstests.config` | 1 conflict marker | File reorganized or absent on main                |

Each conflict is small (1 marker per file) and mechanically resolvable, but
the base divergence is systematic — 103 intervening commits modified these
exact files. Resolving the conflicts produces a working version, but the
resulting branch carries unreviewed intermediate states.

**Current decision**: Do not retry the stale cherry-pick. The current branch
already carries the multi-filesystem framework in `678d56da4` and the tmpfs
integration in `6ee780aa3`; overlay-specific support is implemented as local
configuration and runner changes on top of that framework.

**Overlay-specific xfstests requirements**:
- Overlay is a non-block-backed fs (like tmpfs): it stacks on existing fs.
- `FSTYP=overlay` in xfstests requires `OVL_BASE_FSTYP=ext2` (or tmpfs).
- `prepare.sh` creates and validates the base mount roots; upstream
  `common/overlay` creates the lower/upper/work/mount directories and performs
  the overlay setup.
- The overlay runner invokes upstream `./check -overlay` and applies the local
  Asterinas compatibility shim to `common/rc` before execution.

## PR #3603 (exFAT refactor) — Integrated

PR #3603 (Halifuda, "Refactor the exFAT file system implementation") is
integrated as `c51d9a302` (`refactor exfat implementation`). It modifies the
exFAT implementation, the registry annotation, and the original 55-case list.
The list is now exposed through the filesystem-selectable scaffold as:

- `test/initramfs/src/conformance/xfstests/exfat/run_list/short.list` — 55
  `generic/*` cases from PR3603
- `test/initramfs/src/conformance/xfstests/exfat/run_list/block.list` — no
  exclusions; individual failures remain xfstests results

The exFAT configuration uses `/dev/vdd` and `/dev/vde`, `mkfs.exfat`, empty
`MKFS_OPTIONS` because `mkfs.exfat` does not accept ext2's `-F`, and strict
guest-side mount preparation. Use 8 GiB test/scratch images for this run. The
scaffold does not require further GitHub workflow changes.

## Xfstests Test Mapping (attached to MICRO_FEATURE_INVENTORY.md)

Populated per-tier xfstests overlay test number mapping:
- P0 (11 tests): mount, lookup, readdir, stat, whiteout detection
- P1 (27 tests): copy-up, create/unlink/rename, permissions, file ops
- P2 (15 tests): xino, redirect_dir, ACL, fileattr, nlink, uuid, userxattr
- P3 (~30 tests): index, nfs_export, metacopy, verity, nested overlay

Tests requiring unimplemented features will `_notrun` automatically via
`_require_scratch_overlay_features`. Running `./check -overlay -g auto` is safe
at any implementation stage.

## Xfstests Scaffold and Validation State

The generic runner selects a filesystem directory using `XFSTESTS_FS_TYPE` and
loads its runlist through `XFSTESTS_RUNLIST`. The verified overlay invocation
inside `codex-asterinas-dev` is:

```bash
docker exec -w /root/asterinas codex-asterinas-dev \
  make run_kernel AUTO_TEST=conformance RELEASE=1 MEM=12G \
  CONFORMANCE_TEST_SUITE=xfstests \
  XFSTESTS_FS_TYPE=overlay XFSTESTS_DISK_SIZE=6G \
  XFSTESTS_RUNLIST=/opt/xfstests/overlay/run_list/short.list
```

Overlay is tested using two generated ext2 images, not an overlay block
device. `OVL_BASE_FSTYP=ext2`; upstream `common/overlay` creates the lower,
upper, work, and overlay mount directories below the base mounts. The minimum
exploratory image size is 6 GiB per image; `overlay/001` requires at least 8
GiB free scratch space and is therefore not part of the smoke expectation.

The six-case smoke list started and reached xfstests: 3 cases passed and 3
reported old-overlayfs output mismatches. The packaged full overlay list has
80 cases; a full run reached `overlay/011` before the legacy overlayfs hung.
That run must be classified with preserved QEMU/guest logs and not treated as
a startup failure. Cases marked not-run because of missing `fsgqa`, loopback
support, or scratch-space requirements are expected capability skips.

The old-overlayfs evidence is preserved under
`.agents/components/old-ovfs-baseline-test/`. It contains the raw
`qemu.log` and `qemu-serial.log`, both run lists, the xfstests configuration,
and the Asterinas compatibility shim. The generated 8 GiB TEST and SCRATCH
images remain at `test/initramfs/build/xfstests_test.img` and
`test/initramfs/build/xfstests_scratch.img`; their sizes and SHA-256 values
are recorded in that directory's README rather than copying 16 GiB into Git.

## Lock Topology (for Architect to finalize)

7 lock domains identified (6 from Linux + 1 new for Asterinas):

| Domain  | Primitive                | Crosses BIO | Notes                                                                                             |
| :------ | :----------------------- | :---------- | :------------------------------------------------------------------------------------------------ |
| `DIR`   | `Mutex`                  | Yes         | **NEW** — parent dir consistency (Asterinas VFS doesn't hold this). Outermost overlay-owned lock. |
| `INODE` | `Mutex`                  | Yes         | `ovl_inode->lock` — copy-up state, metadata, readdir cache                                        |
| `WL`    | `Mutex` or `SpinLock`    | No          | `whiteout_lock` — short critical section                                                          |
| `IU`    | `AtomicBool` + waitqueue | No          | `inuse_lock` — mount-time upper/workdir exclusivity                                               |
| `CUL`   | `Mutex`                  | Yes         | Copy-up coordination bit lock (separate from `INODE`)                                             |
| `UPPER` | via underlying fs ops    | Yes         | Implicitly acquired through upper fs calls                                                        |
| `VFS`   | —                        | —           | Linux holds this; Asterinas does NOT. Replaced by `DIR`.                                          |

Static hierarchy: `DIR` > `INODE` > `WL` > `UPPER`; `CUL` orthogonal to all;
`IU` orthogonal (mount-time only). Rename needs dual-`DIR` ordering
(by `Arc::as_ptr()`).

## Next Main-Agent Tasks

1. **Use the preserved old-overlayfs baseline** for comparison. New xfstests
   receipts belong under `.agents/components/<component-id>/`; retain the
   exact command, runlist, guest log, result files, and hang location before
   any rerun overwrites the default artifacts.
2. **Run existing ktests + regression tests on old overlayfs** to establish
   current-state baseline (16 ktests in QEMU, 2 user-space regression tests).
3. **Schedule Architect handoff**: dispatch an Architect packet through
   `$ovfs-subagent` with role `Architect` to internalize the staged priors and
   produce the Global Static Lock Topology + Bi-Directional Traceability
   Matrix. The `protocol/templates/macro_00_global_topology_TEMPLATE.md`
   template is ready.
4. **Decision on scope**: confirm P0+P1 as first wave scope (81 micro-features
   total, 55 in P0+P1). P2/P3 deferred to later waves.

## Unresolved / Open Questions

- The old-overlayfs xfstests baseline has been started; the remaining question
  is how to preserve and schedule its receipts alongside Architect work.
- The `DIR` lock domain needs to be specified at the meso level by Architect.
  Should the Architect do this before or after the Bi-Directional Traceability
  Matrix?
