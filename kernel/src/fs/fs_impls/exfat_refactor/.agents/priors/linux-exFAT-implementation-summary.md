<!-- SPDX-License-Identifier: MPL-2.0 -->

# Linux exFAT Verification Dictionary (VFS Semantic Pipelines)

This document maps the **Asterinas VFS Interfaces** (defined in `ASTERINAS_INTEGRATION_PRIORS.md`) to the **Linux exFAT Dynamic Implementations**. 

Following the principle of prioritizing the most complex edge cases, this skeleton is divided into two categories: **Explicit/Standard VFS Mappings** (where the VFS intent is straightforward and maps cleanly to exFAT physics) and **Implicit/Complex VFS Semantics** (where VFS semantics carry hidden edge cases, atomic guarantees, or concurrency constraints that require deep study of the Linux source code).

## Part 1: Implicit / Complex VFS Semantics (Requires Deep Linux Analysis)
*These areas contain hidden VFS expectations (atomicity, concurrency, strict ordering) that are not obvious from the physical spec alone.*

### 1.1 Allocation & Size Mutation (`fallocate`, `truncate`, `O_APPEND`)
- **`fallocate` Semantic Mapping [MARKED AS DOUBTFUL/PENDING]**: Linux exFAT completely lacks native `fallocate_operations`. `exfat_file_operations` does not implement it. Thus, any semantic like `ZeroRange` or `PunchHoleKeepSize` is either natively rejected by VFS (`-EOPNOTSUPP`) or handled as a slow generic VFS zero-filling write. It does not actively manipulate the Bitmap/FAT via `fallocate` (Needs further verification whether upper VFS handles this via `exfat_setattr` or if we missed a generic block fallback).
- **The `NoFatChain` Flip**: When a file expands and the newly allocated cluster is not physically contiguous with the old chain (i.e., `new_clu != last_clu + 1`):
  1. The code detects the discontinuity and flips the cluster chain flag from `ALLOC_NO_FAT_CHAIN` (1) to `ALLOC_FAT_CHAIN` (0).
  2. **Crucial Backfill (`exfat_chain_cont_cluster`)**: It retrospectively walks the old previously-unmapped contiguous clusters and writes them into the FAT table to link them up explicitly.
  3. The allocation bitmap is updated for the new cluster.
  4. The Stream Extension `NoFatChain` bit and new sizes (`DataLength`) are synced to the Directory Entry via `__exfat_write_inode`.
- **`O_APPEND` & Data Lengths**: Linux manages `DataLength` via `i_size_read(inode)` and `ValidDataLength` via `ei->valid_size`. During `write_inode`, if `DataLength` was expanded but `ValidDataLength` lagged behind (because writes didn't happen yet or crashed), Linux strictly binds them: `if (on_disk_size < ei->valid_size) stream.valid_size = stream.size; else stream.valid_size = ei->valid_size;`. When appending, the directory entry is synced **last** (`__exfat_write_inode`), ensuring clusters are fully locked in the Bitmap/FAT before the directory entry acknowledges the new EOF. If a crash occurs, the file stays at its old size, deliberately "leaking" the new clusters as orphans rather than exposing uninitialized space.

### 1.2 Tree Mutability & Atomicity (`rename`, `create`, `unlink`)
- **Slot Allocation & Deletion (`unlink`, `rmdir`, `mkdir`)**:
  - **Deletion (`unlink`/`rmdir`)**: Linux performs a synchronous `exfat_remove_entries` which iterates over the directory entry triplet and changes the entry type to `TYPE_DELETED` (`0xE5`). Right after the entries are invalidated on disk (`exfat_put_dentry_set`), it frees the clusters via `exfat_free_cluster` which clears the Allocation Bitmap and zeroes the FAT chain.
  - **Allocation (`mkdir`/`create`)**: Linux calls `exfat_find_empty_entry` *before* allocating clusters. It scans the target directory for a contiguous block of empty/deleted (`0xE5`) slots large enough to hold the Triplet + variable Name entries. If the directory is full and `start_clu != EXFAT_EOF_CLUSTER`, it dynamically allocates a new cluster, attaches it to the directory's FAT chain, and expands `i_size`. Once the slots are found, it writes the Triplet (`exfat_init_dir_entry` / `exfat_init_ext_entry`).
- **Cross-Directory `rename` Atomicity**:
  1. Validates the destination (if destination exists and is a directory, it strictly checks `exfat_check_dir_empty`).
  2. **Allocate New**: Uses `exfat_find_empty_entry` in the *new* parent directory to secure destination slots.
  3. **Clone & Modify**: Reads the old Triplet, copies it to the new slots. If it's a file, it adds the `EXFAT_ATTR_ARCHIVE` bit to the clone. It also updates the Stream Extension.
  4. **Invalidate Old**: Synchronously sets the old Triplet slots to `TYPE_DELETED` (`0xE5`).
  5. **Sync Order**: Importantly, it calls `exfat_put_dentry_set` on the *new* directory first, then the *old* directory. This means if a crash occurs mid-way, there might be two directory entries pointing to the same cluster chain (which `fsck` resolves by leaving one in the orphaned state or picking one), but it prevents the cluster from becoming completely orphaned and unreachable.
  6. **Overwrite GC**: If a target file is being overwritten, its old clusters are only freed (`exfat_free_cluster`) at the very end after the new Triplet is safely established.

### 1.3 Directory Iteration & `f_pos` Traversal (`readdir` / `getdents`)
- **Variable-Length Entry Mapping**: Linux exFAT identically maps the VFS `f_pos` (or `cpos`) to the **absolute physical byte offset** inside the directory cluster chain. Because all exFAT entries are exactly 32 bytes, `f_pos` counts up in multiples of 32. When `exfat_readdir` encounters a valid `TYPE_FILE` (0x85) entry, it parses its Stream Extension to find `num_ext`, and safely advances `*cpos = EXFAT_DEN_TO_B(dentry + 1 + num_ext)`. Thus, it natively leaps over the variable parts atomically. If iteration stops, the user-space `f_pos` is safely parked exactly at the 32-byte boundary of the *next* logical entry.
- **Name Hash Collisions (`exfat_find_dir_entry`)**: Linux enforces strict 2-stage verification to bypass linear unicode name comparisons. When scanning for a name, it first checks if the Stream Extension's `name_len` and 16-bit `name_hash` match the target. **Only** if this fast integer match succeeds does it actually extract the subsequent `TYPE_EXTEND` Unicode string buffers and perform a full `exfat_uniname_ncmp`.

### 1.4 Page Cache Block Translation (`bmap` / `get_block`)
- **`NoFatChain` Read Acceleration (`exfat_map_cluster`)**: When resolving a VFS logical block (`iblock`) to a disk sector (`get_block`), Linux leverages the `NoFatChain` flag beautifully. If the flag (`ALLOC_NO_FAT_CHAIN`) is 1, the mapping executes a pure O(1) arithmetic shift (`*clu = ei->start_clu + clu_offset`), entirely short-circuiting the FAT sector read logic. If it is 0 (`ALLOC_FAT_CHAIN`), it must traverse the FAT chain array block by block via `exfat_get_cluster`. This explains the massive sequential IOps performance gain of exFAT and why maintaining the `NoFatChain` state during allocations (Section 1.1) is critical.

### 1.5 Concurrency & Locking Topology (The Linux Baseline)
- **Global Allocator Locks (`sbi->bitmap_lock` & `s_lock`)**: Linux uses coarse-grained synchronization. `sbi->bitmap_lock` strictly protects the Allocation Bitmap to prevent concurrent multi-file cluster allocations from overlapping. `EXFAT_SB(sb)->s_lock` acts as a generic Superblock lock (analogous to a BKL for exFAT) that serializes almost all metadata modifications, including FAT table updates, resolving pathnames (`exfat_find`), and directory entry writes.
- **Directory Mutexes (VFS `i_rwsem` + `s_lock`)**: The VFS layer relies on `inode_lock()` (`i_rwsem`) on the parent directory before invoking `mkdir`, `unlink`, or `rename` to prevent concurrent slot conflicts natively. Within exFAT, it further takes the global `s_lock` to ensure that mapping directory slots (`exfat_find_empty_entry`) and updating the FAT are globally atomized against other directory or File mutators.
- **Inode I/O Locks (`truncate_lock` rw_semaphore)**: Linux exFAT defines a private `truncate_lock` (`struct rw_semaphore`) in its in-memory `exfat_inode_info`. In `exfat_setattr`, truncations/extensions acquire `down_write(&ei->truncate_lock)` before calling `exfat_truncate` to change the file size and FAT chain. Meanwhile, the page cache translation (`exfat_aops.bmap`) acquires `down_read(&ei->truncate_lock)` before invoking `exfat_get_block` so that a logical block mapping won't race against a concurrent truncation freeing those same clusters.


## Part 2: Explicit / Standard VFS Mappings (Straightforward)
*These mappings are generally 1:1 with the VFS intent and require less deep-dive into edge cases.*

### 2.1 Initialization & Global Status (`mount`, `syncfs`, `statfs`)

- **`mount` & Superblock Initialization (`exfat_fill_super`)**:
  - The mount process exclusively acquires a block device reference and reads the boot sector `(sector 0)`.
  - It validates the checksum and loads volume parameters (cluster size, total clusters, FAT length, root directory offset).
  - Pre-computes and caches the up-case table (`exfat_load_upcase_table`) into memory to enable fast case-insensitive name resolution later.
  - Builds the **Volatile Allocation Bitmap**: Unlike FAT32 (which requires scanning the entire FAT table), exFAT uses a dedicated `Bitmap Directory Entry`. The mount process initializes a bit-array from this allocation bitmap to handle immediate allocator queries.
  - Recursively traces the directory structure initially (if needed, e.g., to count `used_clusters` safely without reading all descriptors).

- **Global File System State (`statfs`)**:
  - `f_blocks`: Taken directly from the superblock parameter `sbi->num_clusters - 2`.
  - `f_bfree` and `f_bavail`: Instead of actively counting zeros in the bitmap at runtime, it relies on a cached atomic/protected counter `sbi->used_clusters`. This ensures $O(1)$ fast response.
  - `f_bsize`: Reported natively as `sbi->cluster_size` (which governs allocations).

- **Consistency & Volume Flags (`VOLUME_DIRTY`)**:
  - exFAT has a dedicated `vol_flags` field located in the boot sector (offset `0x6A`), specifically capturing a `VolumeDirty` bit (`0x0002`).
  - During write operations (directory modification, cluster allocation), Linux calls `exfat_set_volume_dirty()`, setting this bit on disk directly to signify the filesystem is "in-flight."
  - Linux `syncfs` does not have a dedicated `exfat_sync_fs` operation hooked into `super_operations`. Instead, metadata and data blocks use the standard VFS page cache flushed by VFS timers. 
  - `umount` (`exfat_put_super`) safely drops the `VolumeDirty` flag via `exfat_clear_volume_dirty`, signals a safe state, and releases the allocation bitmap mapping.
- **`statfs`**: [To be filled: Calculation of available clusters (scanning bitmap vs cached count).]

### 2.2 Name Resolution & Encoding (`lookup`)
- **UTF-16LE to UTF-8 Conversion**: VFS exclusively handles strings as bytes (typically UTF-8). When performing a `lookup` or `create`, Linux dynamically calls the NLS module (`utf8s_to_utf16s`) to translate the desired path segment into UTF-16LE before calculating the 16-bit `name_hash` required to match directory entries. Unrecognized characters fall back to replacement formats.
- **Path Traversal & `i_pos` [MARKED AS DOUBTFUL/PENDING]**: Traversal verifies hash, length, and Unicode characters. To prevent creating full inodes for non-existent paths, Linux introduces an `i_pos` integer key for the cache. `i_pos` uniquely represents an exFAT file by bitwise combining the parent directory's starting cluster and the target entry's byte offset (`(parent_dir_cluster << 32) | index`). This `i_pos` fuels `exfat_build_inode` to instantiate memory models.

### 2.3 Permissions & Ownership Mapping (`chown`, `chmod`)
- **Faux POSIX Permissions**: Since exFAT lacks UID/GID or POSIX modes on-disk, the entire mount dictates ownership dynamically via `fs_uid`, `fs_gid`, `dmask`, and `fmask` global mount options.
- **`chown` / `chmod` Restrictions**: 
  - If a user triggers VFS `setattr` to `chown` a file to an ID other than the currently mounted `fs_uid/fs_gid`, the Linux driver immediately rejects it with `-EPERM`.
  - For `chmod`, the changes are filtered via `exfat_sanitize_mode()`. Only the write bit is actively propagated dynamically to disk by toggling the exFAT DOS `ReadOnly` attribute bit flags; execute semantics remain completely cosmetic based on mount masks.
- **`ReadOnly` Attribute vs `EPERM`**: Flipping `i_mode` write bit triggers an update to the `ATTR_RO` DOS attribute inside the `exfat_dir_entry`. Vice versa, if `ATTR_RO` is found during `exfat_build_inode`, `i_mode` gets stripped of `S_IWUGO`, rendering it read-only for all users dynamically via VFS.

### 2.4 Timestamp & Timezone Translation
- **Timezone Resolution & Encoding**: Unlike standard POSIX which always runs entirely over UTC, exFAT includes explicit `$TimezoneOffset` byte fields alongside each timestamp type. Linux maps the VFS (UTC) timestamps into the local timezone utilizing the global mount `tz_utc/tz_offset` parameter logic. The resulting 10ms resolution elements (`Create10msIncrement`, `Modify10msIncrement`) are calculated dynamically upon flush.

### 2.5 Data Sync (`fsync`)
- **Flush Semantics**: `fsync` in Linux exfat relies heavily on the `__generic_file_fsync()` infrastructure. It flushes dirty page caches. Metadata syncing zeroes in on syncing the directory entry sector (via `exfat_ent_set` or `exfat_update_bhs`) and optionally the FAT sector if new chains were allocated. It does not blindly flush the entire FAT volume to avoid latency bottlenecks.

### 2.6 Unsupported VFS Features (Reusal & EOPNOTSUPP)
- **Symlinks, Hardlinks, and Xattrs**: exFAT intrinsically lacks symlinks and hardlinks. The Linux `exfat_dir_inode_operations` simply does not define a `.symlink` or `.link` operation, natively yielding `-ENOTDIR` or `-EPERM`. Extended attributes (.xattr) are deliberately unimplemented, gracefully returning `-EOPNOTSUPP`.
