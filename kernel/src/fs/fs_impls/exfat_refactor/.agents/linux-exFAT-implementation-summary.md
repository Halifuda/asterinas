# Linux exFAT Implementation Summary

This document summarizes the in-tree Linux exFAT implementation under `fs/exfat/` from three angles:

- `components`: major subsystems and their responsibilities
- `structs` and `functions`: the key runtime and on-disk building blocks
- `interfaces`: how the driver connects to VFS, block I/O, NLS/Unicode, and user space

## Source Map

| File | Main role |
| --- | --- |
| `fs/exfat/exfat_raw.h` | On-disk exFAT layout and constants |
| `fs/exfat/exfat_fs.h` | In-memory structs, helpers, and cross-file declarations |
| `fs/exfat/super.c` | Mount, remount, superblock lifecycle, filesystem registration |
| `fs/exfat/inode.c` | Inode writeback, block mapping, address-space ops, inode cache/hash |
| `fs/exfat/file.c` | File I/O, truncate/expand, ioctl, fsync, mmap integration |
| `fs/exfat/namei.c` | Lookup, create, unlink, mkdir, rmdir, rename, dentry ops |
| `fs/exfat/dir.c` | Directory entry parsing, dentry-set cache, directory iteration, volume label |
| `fs/exfat/fatent.c` | FAT entry access and cluster-chain manipulation |
| `fs/exfat/balloc.c` | Allocation bitmap loading, scanning, bit operations, trim |
| `fs/exfat/cache.c` | Per-inode cluster-chain extent cache |
| `fs/exfat/nls.c` | UTF-16/NLS/UTF-8 conversion and upcase table handling |
| `fs/exfat/misc.c` | Time conversion, checksum helpers, error policy, buffer updates |

## High-Level Architecture

Linux exFAT is organized around three shared state objects:

1. `struct exfat_sb_info`: per-mounted-filesystem state
2. `struct exfat_inode_info`: per-inode state
3. `struct exfat_chain`: a generic representation of a cluster run or cluster chain

The implementation combines two exFAT allocation models:

- `ALLOC_NO_FAT_CHAIN`: contiguous allocation; mapping is arithmetic
- `ALLOC_FAT_CHAIN`: non-contiguous allocation; mapping follows FAT entries

That distinction is the main algorithmic branch in almost every data-path function:

- block lookup
- directory traversal
- allocation
- truncate/free
- dentry-set fetching

## Component Summary

| Component | Key structs | Key functions | Main dependencies | Core algorithm |
| --- | --- | --- | --- | --- |
| Boot/mount validation | `boot_sector`, `exfat_sb_info` | `exfat_read_boot_sector()`, `exfat_verify_boot_region()`, `__exfat_fill_super()` | buffer cache, checksum helpers, upcase/bitmap loaders | Read sector 0, validate signatures and geometry, verify boot checksum region, then build runtime metadata before exposing root |
| Runtime superblock state | `exfat_mount_options`, `exfat_sb_info` | `exfat_init_fs_context()`, `exfat_parse_param()`, `exfat_set_volume_dirty()` | fs_context API, NLS loader, block device | Parse mount options, configure policy and encoding, track bitmap/upcase/FAT geometry, and persist volume-dirty flag in the boot sector |
| Unicode/name layer | `exfat_uni_name` | `exfat_nls_to_utf16()`, `exfat_utf16_to_nls()`, `exfat_uniname_ncmp()`, `exfat_toupper()` | NLS or UTF-8 helpers, upcase table, checksum helpers | Convert user names to UTF-16, uppercase them through the volume upcase table, compute exFAT name hash, compare names case-insensitively |
| Allocation bitmap manager | `exfat_sb_info` (`vol_amap`) | `exfat_load_bitmap()`, `exfat_find_free_bitmap()`, `exfat_set_bitmap()`, `exfat_clear_bitmap()` | root-directory scan, buffer cache, bitmap bitops | Load bitmap from the root directory metadata entry, then search free clusters by word-at-a-time scanning and `ffz()` |
| FAT entry manager | `exfat_chain` | `exfat_ent_get()`, `exfat_ent_set()`, `exfat_alloc_cluster()`, `exfat_free_cluster()` | bitmap layer, buffer cache, discard/TRIM, chain helpers | Maintain consistency between bitmap and FAT, allocate by scanning bitmap, and switch from contiguous mode to FAT-chain mode when allocation stops being contiguous |
| Cluster mapping cache | `exfat_cache`, `exfat_cache_id`, `exfat_inode_info` | `exfat_get_cluster()`, `exfat_cache_lookup()`, `exfat_cache_add()` | FAT reader, inode-local LRU list | Cache contiguous extents of file-cluster to disk-cluster mappings and skip repeated FAT walks |
| Directory entry engine | `exfat_entry_set_cache`, `exfat_dir_entry`, `exfat_dentry` | `exfat_get_dentry_set()`, `exfat_find_dir_entry()`, `exfat_find_empty_entry()` | cluster walker, bitmap validation, NLS/name layer, FAT/bitmap allocators | Treat a file as a validated multi-entry set, scan directory entries as a state machine, and reuse empty-slot hints during create paths |
| Inode/block mapper | `exfat_inode_info`, `exfat_chain` | `exfat_map_cluster()`, `exfat_get_block()`, `__exfat_write_inode()` | FAT/cache/bitmap layers, address-space ops, dentry-set writer | Map logical file blocks to clusters, allocate on demand, and keep directory-entry size/valid-size/start-cluster fields synchronized with data allocation |
| File data path | `exfat_inode_info` | `exfat_file_write_iter()`, `exfat_direct_IO()`, `exfat_cont_expand()`, `__exfat_truncate()` | page cache, block layer, inode mapper, truncate lock | Extend valid data by zero-filling holes before writes, grow cluster chains on expansion, and free tail clusters safely on truncate |
| Namespace/VFS layer | `exfat_dir_entry` | `exfat_lookup()`, `exfat_create()`, `exfat_unlink()`, `exfat_mkdir()`, `exfat_rename()` | dentry ops, name conversion, directory engine, inode builder | Resolve case-insensitive exFAT names, create/remove validated dentry sets, and move or rewrite entry sets for rename |
| Utility/error/time helpers | none central | `__exfat_fs_error()`, `exfat_get_entry_time()`, `exfat_set_entry_time()`, checksum and buffer helpers | mount policy, kernel time helpers, buffer cache | Convert between exFAT timestamps and Unix time, compute boot/dentry checksums, and enforce the selected error policy |

## Important Structs

| Struct | Kind | Purpose | Key dependencies |
| --- | --- | --- | --- |
| `struct boot_sector` | on-disk | Parsed from sector 0 to obtain geometry, FAT placement, root cluster, flags, and size limits | read by `super.c`, validated by checksum/signature checks |
| `struct exfat_dentry` | on-disk | 32-byte directory entry union for file, stream, name, bitmap, upcase, label, and vendor entries | consumed by `dir.c`, `inode.c`, `balloc.c`, `nls.c` |
| `struct exfat_chain` | runtime | Generic descriptor of a chain/run: start cluster, length, allocation mode | shared by FAT, bitmap, directory, and inode code |
| `struct exfat_uni_name` | runtime | UTF-16 file name plus exFAT name hash and length | produced by `nls.c`, used by lookup/create/rename |
| `struct exfat_entry_set_cache` | runtime | Multi-buffer cache for a validated directory-entry set spanning sectors or clusters | central to directory mutation and inode writeback |
| `struct exfat_dir_entry` | runtime | Decoded file/dir metadata extracted from a dentry set | bridge between namespace code and inode construction |
| `struct exfat_mount_options` | runtime | Parsed mount options: ownership, masks, charset, timestamps, discard, error policy | initialized in `super.c`, used across all modules |
| `struct exfat_sb_info` | runtime | Filesystem-wide state: geometry, bitmap, upcase table, cluster search hint, locks, inode hash | core dependency of every subsystem |
| `struct exfat_inode_info` | runtime | exFAT-specific inode state: directory location, start cluster, flags, valid size, hints, cluster cache | central to lookup, mapping, writeback, and truncate |
| `struct exfat_cache` / `struct exfat_cache_id` | runtime | Per-inode extent cache entries for file-cluster to disk-cluster translation | used only in `cache.c`, consumed by `exfat_get_cluster()` |

## Key Function Groups and Algorithms

### 1. Mount and bootstrap

`super.c` owns filesystem bring-up.

- `exfat_read_boot_sector()`
  - Depends on `boot_sector`, `sb_bread()`, block-size calibration.
  - Algorithm: read boot sector, validate magic fields, compute derived geometry, reject inconsistent FAT or data offsets.
- `exfat_verify_boot_region()`
  - Depends on `exfat_calc_chksum32()`.
  - Algorithm: checksum sectors 0..10 with exFAT-specific skipped bytes, then compare against the checksum sector.
- `__exfat_fill_super()`
  - Depends on boot parsing, `exfat_count_num_clusters()`, `exfat_create_upcase_table()`, `exfat_load_bitmap()`.
  - Algorithm: validate boot region, count the root chain first to avoid infinite loops, load upcase and bitmap metadata, then compute used-cluster count.

### 2. Name normalization and matching

`nls.c` and `namei.c` jointly implement case-insensitive lookup.

- `exfat_nls_to_utf16()` / `exfat_utf16_to_nls()`
  - Depend on either kernel NLS tables or UTF-8 conversion helpers.
  - Algorithm: translate between external names and UTF-16, reject or mark lossy conversions, uppercase each code unit through `vol_utbl`, and compute `name_hash`.
- `exfat_toupper()`
  - Depends on the upcase table loaded from disk or the built-in fallback table.
  - Algorithm: O(1) table lookup for UCS-2 code points.
- `exfat_d_hash()` / `exfat_d_cmp()` and UTF-8 variants
  - Depend on name conversion and `exfat_toupper()`.
  - Algorithm: hash and compare names after trimming optional trailing dots and uppercasing each character.

### 3. Allocation bitmap

`balloc.c` is the free-space authority.

- `exfat_load_bitmap()`
  - Depends on root-directory scanning via `exfat_get_dentry()`.
  - Algorithm: find the bitmap dentry in the root directory, read all bitmap sectors, and validate that the bitmap's own clusters are marked allocated.
- `exfat_find_free_bitmap()`
  - Depends on in-memory bitmap buffers and endian helpers.
  - Algorithm: align to a machine word, mask already-scanned bits, load a word, and use `ffz()` to find the first zero bit; wrap once if needed.
- `exfat_count_used_clusters()`
  - Depends on `hweight_long()`.
  - Algorithm: popcount bitmap words, masking the tail word to exclude non-existent clusters.
- `exfat_trim_fs()`
  - Depends on `exfat_find_free_bitmap()` and `sb_issue_discard()`.
  - Algorithm: scan free extents, merge contiguous runs, and discard only runs whose length meets `minlen`.

### 4. FAT and cluster-chain management

`fatent.c` keeps FAT state coherent with the bitmap.

- `exfat_ent_get()` / `exfat_ent_set()`
  - Depend on FAT geometry macros and buffer cache I/O.
  - Algorithm: map cluster number to FAT sector and byte offset, read/write the 32-bit entry, validate reserved/free/bad values, optionally mirror into FAT2.
- `exfat_alloc_cluster()`
  - Depends on bitmap search, bitmap updates, FAT updates, and `bitmap_lock`.
  - Algorithm:
    - choose a hint cluster
    - scan for free clusters in bitmap order
    - keep contiguous allocations in `ALLOC_NO_FAT_CHAIN`
    - if a gap appears, materialize the already-allocated contiguous range into explicit FAT entries and switch to `ALLOC_FAT_CHAIN`
    - update `used_clusters` and `clu_srch_ptr`
- `__exfat_free_cluster()`
  - Depends on bitmap updates, FAT traversal, optional discard.
  - Algorithm:
    - for contiguous mode, clear a known range directly
    - for FAT-chain mode, walk the chain, clear bits, coalesce adjacent clusters for discard, and defend against loops
- `exfat_count_num_clusters()`
  - Depends on FAT walking.
  - Algorithm: return `size` directly for contiguous mode; otherwise traverse FAT until EOF, with loop detection by bounded iteration.

### 5. Directory engine

`dir.c` is the metadata parser and writer.

- `exfat_get_dentry()`
  - Depends on `exfat_find_location()`, bitmap validation, readahead.
  - Algorithm: translate `(chain, entry index)` into `(sector, offset)`, verify the cluster is allocated, then return a pointer into the sector buffer.
- `exfat_get_dentry_set()`
  - Depends on `__exfat_get_dentry_set()` and `exfat_validate_entry()`.
  - Algorithm: gather all sector buffers covering a multi-entry file record, then validate the required ordering `FILE -> STREAM -> NAME... -> secondary`.
- `exfat_find_dir_entry()`
  - Depends on name hashing, `exfat_uniname_ncmp()`, and entry-set walking.
  - Algorithm:
    - optionally start from the `hint_stat` position
    - scan directory entries with a small state machine
    - use stream-entry `name_hash` and `name_len` as a fast filter
    - compare filename-entry fragments only when the filter matches
    - track empty runs to seed later create operations
- `exfat_find_empty_entry()`
  - Depends on `exfat_search_empty_slot()`, directory growth, FAT/bitmap allocation.
  - Algorithm: search for a contiguous empty dentry set; if none exists, append a new cluster to the directory, zero it, and retry.
- `exfat_update_dir_chksum()`
  - Depends on `exfat_calc_chksum16()`.
  - Algorithm: rotate-add checksum across all entries in the set, skipping the checksum field in the primary entry.

### 6. Inode writeback and block mapping

`inode.c` connects exFAT metadata to the page cache and writeback path.

- `__exfat_write_inode()`
  - Depends on dentry-set access and time conversion helpers.
  - Algorithm: rewrite the file and stream entries from in-memory inode state, clamp `valid_size` to on-disk size when needed, update checksum, and flush the whole entry set together.
- `exfat_map_cluster()`
  - Depends on `exfat_get_cluster()` and `exfat_alloc_cluster()`.
  - Algorithm: translate a logical cluster index into a physical cluster; if the file is too short and `create` is set, allocate enough clusters and splice them into the current representation.
- `exfat_get_block()`
  - Depends on `exfat_map_cluster()`, page-cache buffer_heads, and `valid_size`.
  - Algorithm:
    - map the cluster/sector range
    - decide how many contiguous blocks can be returned
    - distinguish fully valid, partially valid, and unwritten regions
    - for a partially valid last block on buffered reads, read the block and zero the unwritten tail
    - for unwritten read-only ranges, clear the mapped state so the page cache sees zeros without disk I/O
- `exfat_iget()` / `exfat_hash_inode()` / `exfat_unhash_inode()`
  - Depend on the per-superblock hash table.
  - Algorithm: key inodes by the on-disk location of the primary directory entry, not by cluster number alone.

### 7. File data path

`file.c` wraps VFS file operations around the inode mapper.

- `exfat_cont_expand()`
  - Depends on `exfat_alloc_cluster()` and last-cluster lookup.
  - Algorithm: grow the allocation first, then enlarge `i_size`; new space is allocated but not yet treated as valid initialized data.
- `__exfat_truncate()`
  - Depends on inode writeback, FAT/bitmap free, and cache invalidation.
  - Algorithm: compute new and physical cluster counts, write the shortened directory entry first, terminate the chain if needed, invalidate cluster-cache hints, then free tail clusters.
- `exfat_extend_valid_size()`
  - Depends on `write_begin`/`write_end`.
  - Algorithm: zero-fill pages from old `valid_size` up to the next write position so reads from sparse logical holes never expose stale disk contents.
- `exfat_file_write_iter()`
  - Depends on generic write helpers and `exfat_extend_valid_size()`.
  - Algorithm: before a write beyond `valid_size`, explicitly zero the gap; then perform the write and sync if required.
- `exfat_direct_IO()`
  - Depends on `exfat_get_block()`.
  - Algorithm: direct write updates `valid_size` after completion; direct read past `valid_size` but before `i_size` is zero-filled in user I/O buffers.

## Interface Summary

### Filesystem registration and mount interfaces

| Interface | Implementation | Role |
| --- | --- | --- |
| `struct file_system_type` | `exfat_fs_type` | Registers the filesystem name `exfat`, fs_context entry point, and kill path |
| `struct fs_context_operations` | `exfat_context_ops` | Mount/remount parameter parsing, tree creation, and reconfiguration |
| `struct super_operations` | `exfat_sops` | Superblock lifecycle: inode allocation/free, writeback, eviction, statfs, shutdown |

### Namespace and dentry interfaces

| Interface | Implementation | Role |
| --- | --- | --- |
| `struct dentry_operations` | `exfat_dentry_ops` | Non-UTF-8 case-insensitive hash/compare/revalidate |
| `struct dentry_operations` | `exfat_utf8_dentry_ops` | UTF-8 hash/compare/revalidate |
| `struct inode_operations` | `exfat_dir_inode_operations` | `create`, `lookup`, `unlink`, `mkdir`, `rmdir`, `rename`, `setattr`, `getattr` |
| `struct file_operations` | `exfat_dir_operations` | `iterate_shared`, directory ioctl, fsync |

### File and memory-mapping interfaces

| Interface | Implementation | Role |
| --- | --- | --- |
| `struct inode_operations` | `exfat_file_inode_operations` | `setattr` and `getattr` for regular files |
| `struct file_operations` | `exfat_file_operations` | read/write iterators, ioctl, mmap, fsync, splice |
| `struct address_space_operations` | `exfat_aops` | folio reads, readahead, writeback, buffered writes, direct I/O, bmap |
| `struct vm_operations_struct` | `exfat_file_vm_ops` | page fault and `page_mkwrite` handling for mmap writes |

### User-visible control interfaces

| Interface | Entry point | Effect |
| --- | --- | --- |
| `FAT_IOCTL_GET_ATTRIBUTES` / `FAT_IOCTL_SET_ATTRIBUTES` | `exfat_ioctl()` | Read/write exFAT DOS attribute bits |
| `FITRIM` | `exfat_ioctl()` -> `exfat_trim_fs()` | Discard free extents |
| `EXFAT_IOC_SHUTDOWN` | `exfat_ioctl()` -> `exfat_force_shutdown()` | Force filesystem shutdown |
| `FS_IOC_GETFSLABEL` / `FS_IOC_SETFSLABEL` | `exfat_ioctl()` | Read/write the volume label dentry |

## Practical Dependency Graph

```text
super.c
  -> nls.c          (upcase table, name case-folding basis)
  -> balloc.c       (allocation bitmap)
  -> fatent.c       (root-chain counting)
  -> inode.c        (root inode setup)

namei.c
  -> nls.c          (path conversion and matching)
  -> dir.c          (find/add/remove dentry sets)
  -> inode.c        (build/hash inode)
  -> fatent.c       (directory growth/free)

file.c / inode.c
  -> cache.c        (logical->physical cluster cache)
  -> fatent.c       (FAT walking, allocation, free)
  -> balloc.c       (bitmap consistency through fatent.c)
  -> dir.c          (write inode back into dentry set)

dir.c
  -> fatent.c       (cluster walking for FAT chains)
  -> balloc.c       (allocated-cluster validation)
  -> misc.c         (checksums, timestamps)
  -> nls.c          (name extraction and conversion)
```

## Design Notes

- The exFAT implementation is metadata-centric: the authoritative file record is the validated dentry set, not just the inode cache.
- `valid_size` is treated separately from `i_size` to prevent stale data exposure in partially initialized tail blocks.
- The allocator optimizes for contiguous runs first and only materializes FAT links when fragmentation forces it.
- Directory lookup is optimized by three hints:
  - `hint_stat` for continuing name scans
  - `hint_femp` for empty-slot reuse
  - `hint_bmap` for last cluster reached during block mapping or readdir
- The volume upcase table is both a correctness dependency and a performance dependency, because name hashing and comparison use it on every lookup path.
