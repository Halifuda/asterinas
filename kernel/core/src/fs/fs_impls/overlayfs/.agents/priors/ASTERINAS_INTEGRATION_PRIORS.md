<!-- SPDX-License-Identifier: MPL-2.0 -->

# Asterinas Integration Priors (For VFS & Substrates)

This document is the "Asterinas OS Reality Dictionary" strictly intended for the **Designer**. It translates the generic OS requirements into the exact constraints the file system module must appease when orchestrated.

**CRITICAL RULE FOR REFACTORING:**
Do **NOT** use the flawed legacy Asterinas filesystem implementation as a reference. Everything written in this dictionary is derived from the generic VFS (`kernel/src/fs/vfs`), the OSTD locks (`ostd::sync`), and stable block device layers.

## 1. Asterinas Lock Primitives & Concurrency Substrate
The Designer must orchestrate locks without violating these hard operational limits:

- **`ostd::sync::SpinLock`**:
  - **Semantics**: Disables thread preemption.
  - **Fatal Constraint**: **STRICTLY FORBIDS** Block I/O, `Bio` calls, sleeping memory allocations, or acquiring any yielding lock (`Mutex`) within its critical section. Doing so will immediately cause a kernel panic/deadlock.
- **`ostd::sync::Mutex` and `RwLock`**:
  - **Semantics**: These are sleep-locks; threads waiting for them will yield to the scheduler.
  - **Constraint**: Safe to hold across Block I/O boundaries, but holding them for too long (e.g., waiting on slow disk sectors) creates severe contention. Revalidate underlying shared state if another thread might have raced during the sleep window.

## 2. Exhaustive BIO (Block I/O) Interfaces (`aster_block`)
The block I/O layer (`aster_block` crate) abstracts raw device communication. Based on how real filesystems (like `ext2`) interact with disks, the Designer should prioritize the high-level `BlockDevice` extension methods rather than building raw BIOs from scratch.

**`BlockDevice` Extension Methods (Common BIO Access)**:
Instead of manual `BioBuilder` construction, the Designer MUST utilize the wrapped block methods deployed for `Arc<dyn BlockDevice>`:
- **Synchronous Block I/O**:
  - `read_blocks(&self, bid: Bid, bio_segment: BioSegment) -> Result<BioStatus, BioEnqueueError>`
  - `write_blocks(&self, bid: Bid, bio_segment: BioSegment) -> Result<BioStatus, BioEnqueueError>`
  - `sync(&self) -> Result<BioStatus, BioEnqueueError>` (Triggers a `BioType::Flush`)
- **Asynchronous Block I/O**:
  - `read_blocks_async(&self, bid: Bid, bio_segment: BioSegment) -> Result<BioWaiter, BioEnqueueError>`
  - `write_blocks_async(&self, bid: Bid, bio_segment: BioSegment) -> Result<BioWaiter, BioEnqueueError>`
  - `write_bytes_async(&self, offset: usize, buf: &[u8]) -> Result<BioWaiter>`

**`VmIo` Trait Implementation for `BlockDevice`**:
The disk also implements `ostd::mm::VmIo`, offering `VmReader` and `VmWriter` bounded I/O:
- `fn read(&self, offset: usize, writer: &mut VmWriter) -> ostd::Result<()>`
- `fn write(&self, offset: usize, reader: &mut VmReader) -> ostd::Result<()>`
*(These are synchronous, blocking operations built on top of `Bio` internally).*

**`BioWaiter` and Pending I/O**:
- `BioWaiter::wait()`: Suspends the current thread (yields) until the hardware interrupts.
- **Blocking Reality**: Submitting synchronous block methods or calling `.wait()` on a `BioWaiter` suspends the thread. The Designer must re-validate any shared state (like allocation block pointers or directory entries) after the thread wakes up if a `Mutex`/`RwLock` was temporarily unlocked.

## 3. Exhaustive PageCacheBackend Interface (`vfs::page_cache`)
For file data caching, Asterinas VFS uses a generic `PageCache`. The Designer must provide a `PageCacheBackend` implementation for inodes that support cached data I/O. This translates generic page offsets into specific physical block manipulations.

**`PageCacheBackend` Trait (in `kernel/src/fs/vfs/page_cache.rs`)**:
- `fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter>;`
  - Maps virtual page index `idx` to absolute on-disk sectors, constructs a BIO (using `BioBuilder`), and returns the asynchronous `BioWaiter`.
- `fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter>;`
  - Same as above, but for writing the memory `frame` to the mapped clusters on disk.
- `fn npages(&self) -> usize;`
  - Returns the ceiling of the file size represented in pages.

**Underlying Types for PageCache**:
- `CachePage`: Represents an in-memory page frame (`Frame<CachePageMeta>`). Contains DMA-safe memory that can be directly appended into a `BioBuilder` segment for zero-copy I/O.
- `BioWaiter`: A token yielded by `kernel/comps/block/src/bio.rs` when an asynchronous I/O is submitted. The VFS layer will call `.wait()` on this token, which will **suspend the thread** until hardware signals completion.

**Design Constraints**:
- The implementer must calculate where `idx` (the Page Cache index) lands in the actual on-disk block mapping.
- Since files might be fragmented, a single `CachePage` (e.g., 4096 bytes) might span multiple disjoint sectors/clusters, requiring careful BIO construction.
- The `read_page_async` and `write_page_async` must not block synchronously for I/O completion; they must return the `BioWaiter` so VFS can sleep.

## 4. Exhaustive FileSystem Trait (VFS Mount & Superblock)
At the top level of the VFS hierarchy, the Designer must orchestrate the overall file system instance by implementing the `FileSystem` trait. This defines the macro-boundaries of the module.

**`FileSystem` Trait (in `kernel/src/fs/vfs/fs_apis/file_system.rs`)**:
- `fn name(&self) -> &'static str;`
  - Required. Identifies the file system type.
- `fn source(&self) -> Option<&str>;`
  - Optional. Provides the mounting source (usually the block device name, e.g., `"/dev/vda1"`).
- `fn sync(&self) -> Result<()>;`
  - Required. Synchronizes the entire file system (all dirty structures and metadata) to the underlying disk. This will likely trigger flush operations on the block device.
- `fn root_inode(&self) -> Arc<dyn Inode>;`
  - Required. **CRITICAL LIFECYCLE RULE:** For each mount, the VFS invokes this method exactly ONCE and eagerly when it creates the mount root. It never defers the lookup to a later path walk. The implementation must synchronously return the root `Inode` object block.
- `fn sb(&self) -> SuperBlock;`
  - Required. Returns the `SuperBlock` metadata representing the current disk capacity geometry (`magic`, block `bsize`, `blocks`, `bfree`, `bavail`, `files`, `ffree`, `namelen`, and the container `DeviceId`). The Designer must ensure these counters are correctly maintained across allocations/deallocations.
- `fn flags(&self) -> FsFlags;`
  - Optional. Returns the current `FsFlags`.
- `fn set_fs_flags(&self, _flags: FsFlags, _data: Option<CString>, _ctx: &Context) -> Result<()>;`
  - Optional. Hook to dynamically alter volume flags (e.g., re-mounting read-only vs read-write).
- `fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats;`
  - Required. Provides the inotify-like event tracking structure.

**Supported `FsFlags` (Bitflags)**:
- `RDONLY` (Read-only), `SYNCHRONOUS` (All writes are synced immediately), `MANDLOCK`, `DIRSYNC` (Directory modifications are strictly synchronous). If read-only mode is specified, the FS must correctly reject mutative `Inode` operations with `EROFS` / `EPERM`.

## 5. Exhaustive VFS Inode Interfaces (`vfs::fs_apis::inode`)
The `Inode` trait bridges VFS path resolution and file actions to the specific file system implementation. A Designer must explicitly coordinate how the filesystem's internal structures and directory entries fulfill these signatures.

**Metadata & Attributes**:
- `fn size(&self) -> usize;`
- `fn resize(&self, new_size: usize) -> Result<()>;`
  - Modifies the valid data length. Must orchestrate allocating/freeing data blocks or clusters if shrinking or expanding.
- `fn metadata(&self) -> Metadata;`
  - Returns `Metadata` containing Dev, Ino, Type, Perms, Link Count, Uid/Gid, Size, Blocks, and Timestamps.
- `ino`, `type_`, `mode`, `set_mode`, `owner`, `set_owner`, `group`, `set_group`.
- Timestamps: `atime`, `mtime`, `ctime` (and their `set_` variants: `set_atime`, `set_mtime`, `set_ctime`).

**I/O (`InodeIo` Trait - Required by `Inode`)**:
- `fn read_at(&self, offset: usize, writer: &mut VmWriter, status_flags: StatusFlags) -> Result<usize>;`
- `fn write_at(&self, offset: usize, reader: &mut VmReader, status_flags: StatusFlags) -> Result<usize>;`
- **`StatusFlags` Handling**:
  - `O_APPEND`: During `write_at`, the write must be evaluated at the very end of the file.
  - `O_SYNC` / `O_DSYNC` / `O_DIRECT`: The implementation must bypass/flush caches appropriately. Direct IO requires physical sector alignment checks.
- `fn fallocate(&self, mode: FallocMode, offset: usize, len: usize) -> Result<()>;`
  - Controls cluster allocation/deallocation explicitly. `FallocMode` variants include:
    - `Allocate`: Allocates disk space within range.
    - `AllocateKeepSize`: Allocates disk blocks/clusters but does not update the file logical size in the directory entry.
    - `PunchHoleKeepSize`: Deallocates clusters (creates a hole) without changing logical size.
    - `ZeroRange`: Converts range to zeros, expanding if necessary.

**Directory & Path Tree Operations**:
- `fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>>;`
  - Fundamental resolution step. Scans this directory's entries matching `name`.
- `fn create(&self, name: &str, type_: InodeType, mode: InodeMode) -> Result<Arc<dyn Inode>>;`
  - Must write the directory entry into the parent and allocate the new target logic. Handling `O_TRUNC` is done prior/post this via explicit file truncation steps.
- `fn mknod(&self, name: &str, mode: InodeMode, type_: MknodType) -> Result<Arc<dyn Inode>>;`
- `fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize>;`
  - Reads directory entries starting at a byte offset, pushing them into the `visitor`.
- `fn link(&self, old: &Arc<dyn Inode>, name: &str) -> Result<()>;`
  - Hardlink creation (If the file system does not support this, VFS allows it to be rejected with `EPERM` / `EOPNOTSUPP`).
- `fn unlink(&self, name: &str) -> Result<()>;`
- `fn rmdir(&self, name: &str) -> Result<()>;`
- `fn rename(&self, old_name: &str, target: &Arc<dyn Inode>, new_name: &str) -> Result<()>;`
  - Atomically moves/renames an entry. Note: Asterinas currently does not pass `RenameFlags` (like `RENAME_EXCHANGE`) down to the specific `Inode::rename` yet, meaning the FS just implements purely positional rename constraints.

**Symlinks (Likely rejected for file systems lacking support using `EOPNOTSUPP` / `EISDIR`)**:
- `fn read_link(&self) -> Result<SymbolicLink>;`
- `fn write_link(&self, target: &str) -> Result<()>;`

**Sync & Hooks**:
- `fn sync_all(&self) -> Result<()>;`
- `fn sync_data(&self) -> Result<()>;`
- `fn open(&self, access_mode: AccessMode, status_flags: StatusFlags) -> Option<Result<Box<dyn FileIo>>>;`
  - VFS hook invoked on file open. Used if the Inode wants to provide a custom VFS File object instead of standard cached accesses.
- `fn page_cache(&self) -> Option<Arc<Vmo>>;`
  - VFS hook returning an optional memory map structure. Most FS implementations return `None` initially to let VFS generate a default `PageCache`.

## 6. Expected Error Variants (`Errno`)
Designers must map errors to standard POSIX-shaped Asterinas OS `Errno` variants (found in `kernel/src/error.rs`):
- `ENOENT`: File or directory not found.
- `ENOTEMPTY`: Expected when `rmdir` is called on a directory containing valid dentries (excluding `.` and `..`).
- `EEXIST`: Expected when creating something that already exists.
- `ENOSPC`: Expected when the file system has no remaining free blocks, clusters, or inodes.
- `EINVAL`: Invalid flags, invalid name hash, or out-of-bounds parameters.
- `EIO`: Underlying `Bio` hardware error.
- `ENOTDIR` / `EISDIR`: Using wrong inode type for directory vs file operations.
- `EPERM` / `EACCES`: General permissions denied.
- `EROFS`: Read-only file system. (MUST be used instead of `EPERM` for read-only volume rejections, e.g. when checking `FsFlags::RDONLY`).
- `ENAMETOOLONG`: File name exceeds the file system's maximum length.
- `EOPNOTSUPP`: Operation not supported (e.g., Hardlinks/Symlinks on file systems that lack support). **WARNING:** The Designer MUST NOT use `ENOSYS`. Asterinas OS explicitly reserves `ENOSYS` for the arch syscall entry code to flag non-existent syscalls, and internal kernel implementations are instructed to refrain from returning it.
