# EXR-INODE-05B Checker Serial Retry

## Runtime Evidence

1. Preflight command:
   `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
   - Result: `no-kvm`
   - Interpretation: QEMU ran without KVM and fell back to TCG.

2. Verification command:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_meta_preserves_validated_file_record_facts'`
   - Result: passed.
   - Runtime mode: TCG-backed QEMU.
   - Observations: build completed, ISO generation completed, and the filtered ktest exited successfully.

3. Verification command:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test root_inode_meta_uses_explicit_synthetic_constructor'`
   - Result: passed.
   - Runtime mode: TCG-backed QEMU.
   - Observations: build completed, ISO generation completed, and the filtered ktest exited successfully.

4. Verification command:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_meta_rejects_directory_length_mismatch'`
   - Result: passed.
   - Runtime mode: TCG-backed QEMU.
   - Observations: build completed, ISO generation completed, and the filtered ktest exited successfully.

5. Verification command:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_meta_accessors_are_pure_read_only_views'`
   - Result: passed.
   - Runtime mode: TCG-backed QEMU.
   - Observations: the run hit a transient crates.io/git TLS fetch warning early in dependency resolution, then recovered and completed successfully with the filtered ktest passing.

## Checker-Owned Coverage

The local `#[ktest]` coverage in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` now covers:

- ordinary metadata-shell construction from validated file-record and chain facts,
- the explicit synthetic root constructor and reserved-root rejection path,
- directory valid-data-length and data-length mismatch rejection,
- repeated read-only accessor stability.

## Verdict

The narrow creator repair resolved the earlier `FatAttr` compatibility blocker. This retry pass is accepted: all four targeted filtered ktests passed under TCG-backed QEMU, and no additional production defect was exposed in the component scope.
