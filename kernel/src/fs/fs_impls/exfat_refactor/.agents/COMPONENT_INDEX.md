<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Component Index

This file is owned by the main agent.
It is the canonical task board for the exFAT multi-agent project: both the refactor itself and the study of how far the workflow can be automated safely.

Allowed states:

- `Planned`
- `Architected`
- `Specified`
- `Implementing`
- `Implemented`
- `Checked`
- `Advised`
- `Accepted`
- `Blocked`

| ID | State | Depends On | Code Budget | Current Owner | Artifacts | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `EXR-BOOT-01` | `Planned` | None | `250-400` | `main-agent` | None yet | Boot region parsing and normalized runtime geometry. |
| `EXR-IO-02` | `Planned` | `EXR-BOOT-01` | `250-350` | `main-agent` | None yet | Metadata I/O and cluster-address translation helpers. |
| `EXR-CHAIN-03` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02` | `300-450` | `main-agent` | None yet | FAT value model and cluster-chain walking. |
| `EXR-DENTRY-04` | `Planned` | `EXR-BOOT-01` | `350-500` | `main-agent` | None yet | Raw dentry model and validated file-record parser. |
| `EXR-INODE-05` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02`, `EXR-CHAIN-03`, `EXR-DENTRY-04` | `350-500` | `main-agent` | None yet | Inode identity and metadata shell for existing on-disk objects. |
| `EXR-SYSROOT-06` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03`, `EXR-DENTRY-04`, `EXR-INODE-05` | `250-400` | `main-agent` | None yet | Root-directory scanner for system entries. |
| `EXR-UPCASE-07` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03`, `EXR-DENTRY-04`, `EXR-SYSROOT-06` | `250-400` | `main-agent` | None yet | Upcase-table loading and case-fold or name-hash support. |
| `EXR-BITMAP-08` | `Planned` | `EXR-IO-02`, `EXR-CHAIN-03`, `EXR-DENTRY-04`, `EXR-SYSROOT-06` | `300-450` | `main-agent` | None yet | Allocation-bitmap loading and in-memory free-space state. |
| `EXR-MOUNT-09` | `Planned` | `EXR-BOOT-01`, `EXR-IO-02`, `EXR-CHAIN-03`, `EXR-INODE-05`, `EXR-UPCASE-07`, `EXR-BITMAP-08` | `300-450` | `main-agent` | None yet | Read-only mount bootstrap and filesystem state object. |
| `EXR-DIR-10` | `Planned` | `EXR-DENTRY-04`, `EXR-INODE-05`, `EXR-UPCASE-07`, `EXR-MOUNT-09` | `400-500` | `main-agent` | None yet | Directory iteration and lookup. |
| `EXR-READ-11` | `Planned` | `EXR-CHAIN-03`, `EXR-INODE-05`, `EXR-MOUNT-09` | `350-500` | `main-agent` | None yet | Regular-file read path and logical-to-physical mapping. |
| `EXR-CREATE-12` | `Planned` | `EXR-CHAIN-03`, `EXR-DENTRY-04`, `EXR-INODE-05`, `EXR-UPCASE-07`, `EXR-BITMAP-08`, `EXR-DIR-10` | `400-550` | `main-agent` | None yet | Namespace create, unlink, mkdir, and rmdir. |
| `EXR-WRITE-13` | `Planned` | `EXR-CHAIN-03`, `EXR-INODE-05`, `EXR-BITMAP-08`, `EXR-MOUNT-09`, `EXR-DIR-10`, `EXR-READ-11`, `EXR-CREATE-12` | `450-650` | `main-agent` | None yet | Allocation growth, write, truncate, rename, and sync. Split further if design exceeds reviewable size. |
