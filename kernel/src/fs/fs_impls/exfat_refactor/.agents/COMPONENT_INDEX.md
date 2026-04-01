<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Component Index

This file is owned by the main agent.
It is the canonical task board for the exFAT multi-agent project: both the refactor itself and the study of how far the workflow can be automated safely.

Allowed states:

- `Planned`
- `Architected`
- `Specified`
- `SerialImplementing`
- `SerialChecked`
- `ConcurrencyImplementing`
- `ConcurrencyChecked`
- `Reviewing`
- `FinalChecked`
- `Accepted`
- `Blocked`

| ID | State | Depends On | Code Budget | Current Owner | Artifacts | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `EXR-BOOT-01` | `Accepted` | None | `250-400` | `main-agent` | [`00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/00_architect.md), [`01_designer_spec.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md), [`02_creator_log.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/02_creator_log.md), [`03_checker_report.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/03_checker_report.md), [`04_advisor_actions.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/04_advisor_actions.md), [`10_creator_log.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/10_creator_log.md), [`11_checker_report.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/11_checker_report.md) | Accepted after repair verification under `no-kvm` TCG. `boot_region_loads_super_block` and `boot_region_rejects_invalid_signature` both exit `0` when run sequentially; parallel `cargo osdk test` runs are still avoided because of an OSDK `grub.rs` directory-concurrency panic. |
| `EXR-IO-02` | `Accepted` | `EXR-BOOT-01` | `250-350` | `main-agent` | [`00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/00_architect.md), [`01_designer_spec.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/01_designer_spec.md), [`02_creator_log.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/02_creator_log.md), [`03_checker_report.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/03_checker_report.md) | Accepted after sequential checker validation under `no-kvm` TCG. `cargo osdk test metadata_reads_unaligned_slice_across_block_boundary` and `cargo osdk test exfat_refactor::tests` both exit `0`; the component remains pure and read-only, so no separate concurrency pass is required. |
| `EXR-BOOTTYPE-14` | `Accepted` | `EXR-BOOT-01` | `120-220` | `main-agent` | [`00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/00_architect.md), [`01_designer_spec.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/01_designer_spec.md), [`10_creator_serial.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/10_creator_serial.md), [`11_checker_serial.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/11_checker_serial.md), [`30_reviewer_report.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/30_reviewer_report.md), [`31_checker_final.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/31_checker_final.md), [`32_checker_final_retry.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOTTYPE-14/32_checker_final_retry.md) | Typed validation boundary is explicit and the reviewer-tightened slice now has a clean post-review rerun under `no-kvm` TCG. The component is accepted. |
| `EXR-FATVAL-03A` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02` | `180-260` | `main-agent` | None yet | FAT entry value model and next-cluster decoding only. Ready-now wave with `EXR-DENTRY-04A`. |
| `EXR-CHAIN-03B` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02`, `EXR-FATVAL-03A` | `220-320` | `main-agent` | None yet | Cluster-chain walking over contiguous and FAT-backed cases. Split away from raw FAT entry decoding to keep method count bounded. |
| `EXR-DENTRY-04A` | `Planned` | `EXR-BOOT-01` | `200-280` | `main-agent` | None yet | Raw 32-byte dentry layout and typed single-entry decoding. Ready-now wave with `EXR-FATVAL-03A`. |
| `EXR-FILESET-04B` | `Planned` | `EXR-BOOT-01`, `EXR-DENTRY-04A` | `220-320` | `main-agent` | None yet | Validated multi-entry file-record parser and name-entry aggregation. Split away from raw entry decoding to isolate the record-state machine. |
| `EXR-INOKEY-05A` | `Planned` | `EXR-BOOT-01`, `EXR-FILESET-04B` | `180-240` | `main-agent` | None yet | Inode identity key and on-disk location descriptors for existing objects. Ready-now wave with `EXR-CHAIN-03B` once `EXR-FILESET-04B` lands. |
| `EXR-INODE-05B` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, `EXR-INOKEY-05A` | `220-320` | `main-agent` | None yet | Read-only inode metadata shell built from parsed file records and chain facts. VFS behavior stays out of scope. |
| `EXR-SYSROOT-06` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, `EXR-INODE-05B` | `220-300` | `main-agent` | None yet | Root-directory scanner for system entries. Depends on parsed file sets and minimal inode metadata, not on later directory APIs. |
| `EXR-UPCASE-07` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, `EXR-SYSROOT-06` | `220-300` | `main-agent` | None yet | Upcase-table loading and case-fold or name-hash support. Ready to run in parallel with `EXR-BITMAP-08` after `EXR-SYSROOT-06`. |
| `EXR-BITMAP-08` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, `EXR-SYSROOT-06` | `220-320` | `main-agent` | None yet | Allocation-bitmap loading and in-memory free-space state. Ready to run in parallel with `EXR-UPCASE-07` after `EXR-SYSROOT-06`. |
| `EXR-MOUNT-09` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-INODE-05B`, `EXR-UPCASE-07`, `EXR-BITMAP-08` | `220-320` | `main-agent` | None yet | Read-only mount bootstrap and filesystem state object. Keeps mount sequencing separate from boot parsing and inode shaping. |
| `EXR-DIR-10` | `Planned` | `EXR-DENTRY-04A`, `EXR-FILESET-04B`, `EXR-INODE-05B`, `EXR-UPCASE-07`, `EXR-MOUNT-09` | `240-340` | `main-agent` | None yet | Directory iteration and lookup. Depends on parsed directory records rather than redoing dentry decoding inline. |
| `EXR-READ-11` | `Planned` | `EXR-CHAIN-03B`, `EXR-INODE-05B`, `EXR-MOUNT-09` | `220-320` | `main-agent` | None yet | Regular-file read path and logical-to-physical mapping for existing files. Ready to run in parallel with early `EXR-DIR-10` work once `EXR-MOUNT-09` lands. |
| `EXR-CREATE-12` | `Planned` | `EXR-CHAIN-03B`, `EXR-DENTRY-04A`, `EXR-FILESET-04B`, `EXR-INODE-05B`, `EXR-UPCASE-07`, `EXR-BITMAP-08`, `EXR-DIR-10` | `260-360` | `main-agent` | None yet | Namespace create, unlink, mkdir, and rmdir. Still a likely future split point if design grows across too many mutation cases. |
| `EXR-WRITE-13` | `Planned` | `EXR-CHAIN-03B`, `EXR-INODE-05B`, `EXR-BITMAP-08`, `EXR-MOUNT-09`, `EXR-DIR-10`, `EXR-READ-11`, `EXR-CREATE-12` | `280-380` | `main-agent` | None yet | Allocation growth, write, truncate, rename, and sync. This remains an umbrella mutation area and should be split again during architecting before design starts. |
