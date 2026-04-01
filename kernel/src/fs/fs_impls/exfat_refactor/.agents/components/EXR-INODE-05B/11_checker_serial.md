# EXR-INODE-05B Checker Serial

## Runtime Evidence

1. Preflight command:
   `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
   - Result: `no-kvm`
   - Interpretation: QEMU was expected to fall back to TCG rather than KVM.

2. Verification command:
   `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_meta_preserves_validated_file_record_facts'`
   - Result: build failure before any ktest executed.
   - QEMU observation: the run forwarded guest ports, so the test harness started, but the build did not reach runtime execution.
   - Failure class: production build defect in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`.
   - Observed errors:
     - `bitflags!` on `FatAttr` conflicts with the derived `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq` implementations at `inode.rs:20-31`.
     - `FatAttr::from_bits_retain` is unavailable at `inode.rs:97` under the current macro expansion.

## Checker-Owned Coverage Added

- `inode_meta_preserves_validated_file_record_facts`
- `root_inode_meta_uses_explicit_synthetic_constructor`
- `inode_meta_rejects_directory_length_mismatch`
- `inode_meta_accessors_are_pure_read_only_views`

These local `#[ktest]` cases were added in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs:350-495` to cover ordinary metadata-shell construction, the explicit root special case, directory-length mismatch rejection, and accessor stability.

## Verdict

The component is blocked on the inode production compile defect above. I did not run the remaining filtered ktests, and I did not modify production code to work around the defect in this checker pass.
