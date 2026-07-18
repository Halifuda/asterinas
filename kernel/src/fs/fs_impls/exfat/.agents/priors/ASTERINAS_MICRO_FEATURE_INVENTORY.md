<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Micro-Feature Inventory

This document is the exhaustive micro-feature inventory scaffold for the refactored exFAT implementation.
It is intentionally kept at the scaffold level.
Its purpose is to provide a stable place to accumulate source-backed micro-features without prematurely implying topology, pass slicing, or false completeness.

## 1. Inventory Schema

Each micro-feature record should be captured with the following fields:

| Field | Meaning |
| :--- | :--- |
| `Feature ID` | Stable identifier for the feature record. |
| `Feature Name` | Short factual label for the behavior or invariant. |
| `Layer` | One of `Physical/On-Disk`, `VFS/Interface`, or `BIO Substrate`. |
| `Trigger / Entry Surface` | The syscall, VFS hook, mount path, metadata scan, or internal transition that activates this feature. |
| `Primary State / Objects` | The on-disk records, inode fields, dentries, page-cache state, bitmap/FAT state, or block-device objects involved. |
| `Required Invariant / Guarantee` | The exact property that must hold true. |
| `Failure / Edge Conditions` | The corruption, ENOSPC, EIO, invalid-name, or recovery-sensitive conditions tied to this feature. |
| `Mount / Policy Sensitivity` | Any mount option, encoding mode, timezone policy, or admin setting that changes semantics. |
| `Primary Source Anchors` | Exact prior/source references that justify the record. |
| `Ownership Notes` | Reserved for later topology mapping; keep factual and non-prescriptive. |

## 2. Physical / On-Disk Layer

This section is reserved for source-backed micro-features about the durable exFAT layout and on-disk state machines.
Populate it incrementally from the Microsoft spec and Linux verification notes.

### Cluster A. Mount / Superblock / Global Status

#### Record `INV-PHY-001`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-001` |
| `Feature Name` | `Boot region validation and parameter load at mount` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Initial mount path validating boot-sector state before accepting volume geometry. |
| `Primary State / Objects` | Main and Backup Boot regions, their respective boot-checksum state, cluster-size and FAT geometry fields, and `FirstClusterOfRootDirectory`. |
| `Required Invariant / Guarantee` | Before using the contents of either Main or Backup Boot Sector, implementations must validate the respective boot checksum and field ranges; the Backup Boot region exists as a recovery aid, but its `VolumeFlags` and `PercentInUse` fields are treated as stale. Accepted runtime geometry must match the validated boot-region metadata. |
| `Failure / Edge Conditions` | Invalid checksum, malformed field ranges, or unreadable boot-region state must prevent a normal mounted instance from being established unless a separately validated recovery source is accepted. |
| `Mount / Policy Sensitivity` | Applies on every mount regardless of option set; later policy choices rely on this accepted geometry. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `## 3 Main and Backup Boot Regions`, `### 3.1 Main and Backup Boot Sector Sub-regions`, `### 3.4 Main and Backup Boot Checksum Sub-regions`, `#### 3.1.10 FirstClusterOfRootDirectory Field` |
| `Ownership Notes` | Reserved. Treat as mount-time physical truth, not as a topology hint. |

#### Record `INV-PHY-002`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-002` |
| `Feature Name` | `Allocation bitmap is the free-space truth source` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Mount-time free-space initialization and later allocator / free-cluster accounting. |
| `Primary State / Objects` | Allocation Bitmap directory entry, allocation bitmap contents, volatile in-memory bitmap image, and `used_clusters` accounting state. |
| `Required Invariant / Guarantee` | Free-space state must be seeded from the on-disk allocation bitmap and then kept consistent with later allocation / free events rather than recomputing from unrelated metadata on each query. |
| `Failure / Edge Conditions` | Bitmap corruption or recovery-sensitive inconsistencies may force recount or mount failure handling; normal `statfs` reporting must not require a fresh full scan. |
| `Mount / Policy Sensitivity` | Not option-dependent, but later runtime reporting and allocator behavior depend on the same seeded bitmap view. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.1 Allocation Bitmap Directory Entry`, `#### 7.1.5 Allocation Bitmap` |
| `Ownership Notes` | Reserved. Record the authoritative free-space source only; do not infer allocator topology here. |

#### Record `INV-PHY-003`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-003` |
| `Feature Name` | `VolumeDirty marks in-flight versus quiesced global state` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Mutating filesystem activity, global sync / unmount completion, and mount-time interpretation of on-disk volume state. |
| `Primary State / Objects` | Boot-sector `VolumeFlags` field, `VolumeDirty` bit, and mount-visible volume state. |
| `Required Invariant / Guarantee` | The durable dirty-state marker must reflect whether the volume is still in-flight versus cleanly quiesced, and updates to that marker must be tied to filesystem-wide state transitions rather than to a single inode view. |
| `Failure / Edge Conditions` | Crash, forced shutdown, or incomplete metadata flush can leave the dirty bit asserted; clean teardown must be able to clear it. |
| `Mount / Policy Sensitivity` | Not directly option-driven, but later global-state handling depends on this persisted flag. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 3.1.13 VolumeFlags Field`, `##### 3.1.13.2 VolumeDirty Field` |
| `Ownership Notes` | Reserved. Keep this as a volume-state fact, not as a lock or owner claim. |

#### Record `INV-PHY-004`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-004` |
| `Feature Name` | `VolumeFlags also carries media-failure and clear-before-modify state` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Mount-time interpretation of boot-sector global status and any later media-failure or recovery-sensitive transitions. |
| `Primary State / Objects` | Boot-sector `VolumeFlags` field, `MediaFailure` bit, `ClearToZero` bit, and the stale-copy rule for Backup Boot Sector `VolumeFlags`. |
| `Required Invariant / Guarantee` | `MediaFailure` records whether unresolved media access failures have been observed, while `ClearToZero` records a pre-modification clearing requirement defined by the spec. When consulting the Backup Boot Sector, implementations must treat `VolumeFlags` as stale rather than authoritative current state. |
| `Failure / Edge Conditions` | Failed media accesses may require asserting `MediaFailure`; recovery workflows that resolve known failures may later clear it. `ClearToZero` does not carry ordinary steady-state meaning but becomes significant before metadata, directory, or file modification when set. |
| `Mount / Policy Sensitivity` | Not mount-option-driven; this is persisted global-status and recovery metadata from the physical boot region. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 3.1.13 VolumeFlags Field`, `##### 3.1.13.3 MediaFailure Field`, `##### 3.1.13.4 ClearToZero Field` |
| `Ownership Notes` | Reserved. Keep this as a physical-state fact and recovery boundary, not as a topology decision. |

### Cluster B. Lookup / Name Encoding / Dentry Coherence

#### Record `INV-PHY-005`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-005` |
| `Feature Name` | `Up-case Table is the durable case-folding truth source` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Mount-time load of the on-disk case-folding table and later case-insensitive name matching. |
| `Primary State / Objects` | Up-case Table directory entry in the root directory, its `TableChecksum`, `FirstCluster`, `DataLength`, and the persisted Up-case Table stored in the cluster heap. |
| `Required Invariant / Guarantee` | Implementations must verify the `TableChecksum` before using the Up-case Table, and the table remains the durable exFAT source for case-insensitive yet case-preserving filename semantics. |
| `Failure / Edge Conditions` | Invalid checksum or malformed table metadata must prevent trusting the loaded table for normal case-folding behavior. |
| `Mount / Policy Sensitivity` | Not mount-option-driven at the physical layer, though later lookup behavior depends on the accepted table contents. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.2 Up-case Table Directory Entry`, `#### 7.2.2 TableChecksum Field`, `#### 7.2.5 Up-case Table` |
| `Ownership Notes` | Reserved. Keep this as a durable naming-state fact, not as a topology or helper-layout hint. |

### Cluster C. Allocation / Size Mutation / Write Ordering

#### Record `INV-PHY-006`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-006` |
| `Feature Name` | `Stream Extension encodes allocation topology and initialized-data extent` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Interpreting or persisting a file's Stream Extension directory entry during allocation, growth, shrink, and read/write state updates. |
| `Primary State / Objects` | Stream Extension `AllocationPossible`, `NoFatChain`, `ValidDataLength`, `FirstCluster`, and `DataLength` fields. |
| `Required Invariant / Guarantee` | Stream Extension entries always carry `AllocationPossible = 1`. `ValidDataLength` records how far user data has actually been written and must remain within `0..=DataLength`; for directories it must equal `DataLength`. Reads beyond `ValidDataLength` must return zeroes rather than undefined media contents. |
| `Failure / Edge Conditions` | A malformed stream state where `ValidDataLength > DataLength` is invalid; directory streams cannot carry a divergent `ValidDataLength`. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 7.6.2.1 AllocationPossible Field`, `##### 7.6.2.2 NoFatChain Field`, `#### 7.6.5 ValidDataLength Field`, `#### 7.6.6 FirstCluster Field`, `#### 7.6.7 DataLength Field` |
| `Ownership Notes` | Reserved. Treat this as persisted stream-state truth, not as an implementation layout hint. |

### Cluster D. Directory Lifecycle / Tree Mutability

#### Record `INV-PHY-007`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-007` |
| `Feature Name` | `File and directory names live in consecutive directory-entry sets guarded by set checksums` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Creating, cloning, invalidating, or re-reading file and directory entry sets inside a directory stream. |
| `Primary State / Objects` | Primary directory entry, `SecondaryCount`, `SetChecksum`, the mandatory Stream Extension entry, and the consecutive File Name entries that follow it. |
| `Required Invariant / Guarantee` | A file or directory entry set is a contiguous primary-plus-secondary unit: `SecondaryCount` states how many secondary entries immediately follow the primary, `SetChecksum` covers the whole set, the Stream Extension must immediately follow the File entry, and File Name entries must then follow consecutively. Implementations must verify the set checksum before trusting the rest of the set. |
| `Failure / Edge Conditions` | A broken consecutive layout or invalid set checksum makes the directory-entry set untrustworthy for normal mutation or lookup use. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 6.3.2 SecondaryCount Field`, `#### 6.3.3 SetChecksum Field`, `### 7.6 Stream Extension Directory Entry`, `#### 7.7.3 FileName Field` |
| `Ownership Notes` | Reserved. This is a persistent entry-set fact, not a future component split. |

### Cluster F. Permissions / Ownership / Timestamp / Timezone

#### Record `INV-PHY-008`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-008` |
| `Feature Name` | `File entry sets persist DOS-style attributes and local-time timestamp triples` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Creating or updating a file/directory entry set's attribute and timestamp metadata on disk. |
| `Primary State / Objects` | `FileAttributes`, `CreateTimestamp` / `Create10msIncrement` / `CreateUtcOffset`, `LastModifiedTimestamp` / `LastModified10msIncrement` / `LastModifiedUtcOffset`, and `LastAccessedTimestamp` / `LastAccessedUtcOffset`. |
| `Required Invariant / Guarantee` | exFAT persists DOS-style attribute flags and stores timestamps as local time with explicit UTC-offset side fields. Creation time is set at creation, last-modified time is updated after content or length changes, and last-accessed time may be updated on reads and must be updated after content/length changes. |
| `Failure / Edge Conditions` | Timestamp fields have fixed resolution and range limits, including two-second base resolution plus optional 10ms increments, so implementations must not assume arbitrary POSIX timestamp fidelity on disk. |
| `Mount / Policy Sensitivity` | Not directly mount-option-driven at the physical layer, though later VFS translation depends on timezone policy. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.4.4 FileAttributes Field`, `#### 7.4.5 CreateTimestamp, Create10msIncrement, and CreateUtcOffset Fields`, `#### 7.4.6 LastModifiedTimestamp, LastModified10msIncrement, and LastModifiedUtcOffset Fields`, `#### 7.4.7 LastAccessedTimestamp and LastAccessedUtcOffset Fields`, `#### 7.4.8 Timestamp Fields`, `#### 7.4.10 UtcOffset Fields` |
| `Ownership Notes` | Reserved. Keep this as persisted metadata truth, not as a topology hint. |

### Cluster G. Administrative ABI / Unsupported / Refusal Boundary

#### Record `INV-PHY-009`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-009` |
| `Feature Name` | `Volume label and volume GUID live as dedicated administrative directory entries` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Reading or rewriting on-disk volume-identity metadata for administrative queries and updates. |
| `Primary State / Objects` | Volume Label directory entry, `VolumeLabel` field contents, and the Volume GUID directory entry. |
| `Required Invariant / Guarantee` | exFAT stores administrative identity metadata in dedicated directory entries rather than synthesizing it from ordinary inode names. Volume labels and volume GUIDs therefore have their own persistent record formats and update surfaces. |
| `Failure / Edge Conditions` | Administrative metadata conversion or validation failures must not be mistaken for ordinary namespace lookup failures on file entries. |
| `Mount / Policy Sensitivity` | Not directly mount-option-driven at the physical layer. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.3.3 VolumeLabel Field`, `### 7.5 Volume GUID Directory Entry` |
| `Ownership Notes` | Reserved. This is durable administrative metadata, not a topology hint. |

### Cluster H. Consistency / Recovery / Anomaly Surface

#### Record `INV-PHY-010`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-PHY-010` |
| `Feature Name` | `Recommended write ordering uses VolumeDirty as the on-disk consistency bracket` |
| `Layer` | `Physical/On-Disk` |
| `Trigger / Entry Surface` | Multi-step on-disk updates to directory entries, FAT state, and other mutable filesystem structures. |
| `Primary State / Objects` | `VolumeDirty`, ordered metadata/data update sequence, and final dirty-bit clear step when the volume was previously clean. |
| `Required Invariant / Guarantee` | The exFAT specification frames consistency-sensitive updates with `VolumeDirty`: set it before the ordered mutation sequence, perform the required writes in sequence, and clear it afterward when the volume entered the sequence clean. |
| `Failure / Edge Conditions` | Interrupting the sequence can leave the volume marked dirty and expose partially completed metadata transitions that later recovery tooling must interpret. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.2 VolumeDirty Field`, `### 8.1 Recommended Write Ordering` |
| `Ownership Notes` | Reserved. This is a persistence-ordering fact, not a topology hint. |

## 3. VFS / Interface Layer

This section is reserved for source-backed micro-features about user-visible semantics, VFS hooks, namespace behavior, mount-option effects, and refusal boundaries.
Populate it incrementally from the Linux verification notes and Asterinas integration priors.

### Cluster A. Mount / Superblock / Global Status

#### Record `INV-VFS-001`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-001` |
| `Feature Name` | `Mount option defaults and remount mutability boundary` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Initial mount option parsing and remount / reconfigure requests. |
| `Primary State / Objects` | Parsed mount options, cached dentry and inode semantics, block-device discard capability, and remount request parameters. |
| `Required Invariant / Guarantee` | Options whose semantics are explicitly recorded in the current Linux prior as cached in dentries or inodes must remain stable across remount, while `discard` is the only explicitly mutable option in this record and still requires discard-capable media when enabled at remount time. |
| `Failure / Edge Conditions` | Remount must reject incompatible semantic changes with `-EINVAL`; enabling `discard` on a nondiscard-capable device at remount is also rejected with `-EINVAL`. |
| `Mount / Policy Sensitivity` | `errors=remount-ro`, `allow_utime`, `iocharset`, `discard`, and `keep_last_dots` are explicitly covered by this record. `zero_size_dir` remains a mount-time semantic option in the current priors, but this record does not claim a remount mutability rule for it. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status` |
| `Ownership Notes` | Reserved. This record captures semantic stability boundaries only. |

#### Record `INV-VFS-002`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-002` |
| `Feature Name` | `Superblock counters and statfs reflect cached cluster accounting` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `statfs` / superblock query paths after mount and after allocation-state changes. |
| `Primary State / Objects` | `num_clusters`, `cluster_size`, `used_clusters`, exported superblock counters, and VFS-visible free-space reporting. |
| `Required Invariant / Guarantee` | Reported block counts and free-space counters must derive from accepted mount geometry plus incrementally maintained used-cluster state, not from a fresh full bitmap scan on every query. |
| `Failure / Edge Conditions` | Corruption-recovery paths may trigger recount, but steady-state reporting must continue to expose coherent counters. |
| `Mount / Policy Sensitivity` | Depends on the mount-established geometry and allocator-maintained `used_clusters`, not on a user option toggle. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` |
| `Ownership Notes` | Reserved. Keep the record at the reporting-contract level and avoid assigning a future owner. |

#### Record `INV-VFS-003`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-003` |
| `Feature Name` | `Online discard is opportunistic and can downgrade at runtime` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Mount with `discard`, remount toggles, and runtime free-cluster release paths. |
| `Primary State / Objects` | `discard` mount option state, block-device discard capability, free-cluster ranges, and runtime in-memory discard enablement. |
| `Required Invariant / Guarantee` | Online discard is an opportunistic free-space hint layered on top of cluster release; it must never be treated as the only correctness path for freeing space, and the runtime may disable it after device refusal without invalidating ordinary free-space semantics. |
| `Failure / Edge Conditions` | Mount on nondiscard-capable media warns and disables online discard instead of failing outright; runtime `-EOPNOTSUPP` responses disable future online discards; remount enablement on unsupported media is rejected. |
| `Mount / Policy Sensitivity` | Fully controlled by the `discard` policy plus observed block-device capability. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.7 Administrative & Maintenance ABI` |
| `Ownership Notes` | Reserved. Treat as a policy-sensitive runtime behavior, not as a trim-topology decision. |

#### Record `INV-VFS-004`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-004` |
| `Feature Name` | `Asterinas mount lifecycle must eagerly expose root inode and global sync state` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Filesystem instantiation through the Asterinas `FileSystem` trait, including initial mount root creation and filesystem-wide sync / superblock queries. |
| `Primary State / Objects` | `FileSystem::root_inode()`, `FileSystem::sync()`, `FileSystem::sb()`, current `FsFlags`, and exported root / superblock objects. |
| `Required Invariant / Guarantee` | Each mount must synchronously provide a root inode exactly once at mount-root creation time, expose coherent superblock counters, and support filesystem-wide sync semantics that flush dirty global state to backing storage. |
| `Failure / Edge Conditions` | Read-only flags must reject mutating operations with the expected VFS errors; sync and superblock reporting still need to reflect a coherent mounted instance. |
| `Mount / Policy Sensitivity` | Sensitive to filesystem flags such as `RDONLY`, `SYNCHRONOUS`, and `DIRSYNC`. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` |
| `Ownership Notes` | Reserved. This is an integration-boundary fact and should not be turned into an owner claim here. |

### Cluster B. Lookup / Name Encoding / Dentry Coherence

#### Record `INV-VFS-005`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-005` |
| `Feature Name` | `iocharset selects the lookup codec and dcache comparison pipeline` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `lookup` / `create` path conversion of VFS byte strings into exFAT name matching inputs. |
| `Primary State / Objects` | `iocharset` mount option, UTF-8 or NLS conversion path, UTF-16LE name materialization, and the dentry hash/compare operators chosen for the mount. |
| `Required Invariant / Guarantee` | The mount's chosen `iocharset` determines both the user-visible name codec and the dcache comparison engine: `iocharset=utf8` uses the UTF-8-specific path, while other `iocharset=` values route through the named NLS conversion path. |
| `Failure / Edge Conditions` | Unrecognized or unrepresentable characters fall back through the conversion machinery rather than bypassing the exFAT name-hash pipeline. |
| `Mount / Policy Sensitivity` | Fully sensitive to `iocharset`, including the special UTF-8 path versus NLS-backed paths. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.2 Name Resolution & Encoding` |
| `Ownership Notes` | Reserved. This is a mount-sensitive lookup fact, not a component boundary claim. |

#### Record `INV-VFS-006`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-006` |
| `Feature Name` | `Stable inode identity follows directory-entry location rather than transient pathname spelling` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `lookup`, `create`, `mkdir`, `readdir`, and rename paths that instantiate, rediscover, or rebind inode identity. |
| `Primary State / Objects` | Stable inode identity key, parent directory-cluster anchor, directory-entry slot/index, root-directory special case, and any inode-cache state used to avoid duplicate instantiation. |
| `Required Invariant / Guarantee` | Non-root inode identity must be reconstructible from the durable directory-entry location rather than from a transient pathname spelling or a byte offset. Lookup-family paths, directory iteration, and rename-driven rebinding must preserve or update that identity coherently when an entry moves. |
| `Failure / Edge Conditions` | Reusing stale location-derived identity after entry-slot movement would alias the wrong inode object; the root object may remain a special-case identity anchor. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. Linux currently realizes this through `i_pos`, but this row records only the location-based identity rule. |

#### Record `INV-VFS-007`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-007` |
| `Feature Name` | `Create-oriented namespace resolution cannot trust stale miss state once aliases may materialize` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Create-oriented name resolution and ordinary reuse of cached miss state, if the namespace carrier caches negative lookups at all. |
| `Primary State / Objects` | Cached-miss state if present, parent-namespace coherence/version marker, long-name plus 8.3 alias materialization, and creation intent. |
| `Required Invariant / Guarantee` | If the namespace carrier caches negative lookup results, create-oriented resolution must bypass or invalidate a previously cached miss so the exact user-supplied spelling is re-evaluated. Outside create-oriented resolution, cached miss reuse must remain bounded by a parent-namespace coherence signal rather than assumed timeless. |
| `Failure / Edge Conditions` | A stale cached miss can hide a newly materialized alias or outdated namespace state, so any carrier with negative lookup caching needs an explicit invalidation or revalidation rule. |
| `Mount / Policy Sensitivity` | Not directly mount-option-driven; this is namespace-coherence behavior. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. Linux currently realizes this through dentry-cache revalidation, but this row records the namespace-coherence pressure rather than a prescribed target carrier. |

#### Record `INV-VFS-008`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-008` |
| `Feature Name` | `Name-match prefilter and comparison are encoding-aware, case-folded, and trailing-dot-sensitive` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Name-match prefiltering and candidate comparison before the deeper on-disk directory walk. |
| `Primary State / Objects` | Candidate lookup names, selected encoding path, exFAT Up-case Table or UTF-8 folding path, and trailing-dot normalization state. |
| `Required Invariant / Guarantee` | Name hashing and comparison must be encoding-aware and case-insensitive. `NameHash` is only a prefilter: candidate matches still require full up-cased filename comparison before a lookup is considered resolved. In NLS mode folding goes through the exFAT Up-case Table; in UTF-8 mode BMP code points fold through the UTF-8-specific path. Trailing dots are stripped for comparison unless `keep_last_dots` is enabled. |
| `Failure / Edge Conditions` | Hash agreement alone is insufficient because collisions still require full filename comparison. The current Linux prior's UTF-8-specific path has more limited supplementary-plane handling because the exFAT Up-case Table covers only BMP code points. |
| `Mount / Policy Sensitivity` | Sensitive to both `iocharset` and `keep_last_dots`. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.2 Up-case Table Directory Entry`, `#### 7.6.4 NameHash Field`, `#### 7.7.3 FileName Field` |
| `Ownership Notes` | Reserved. This is a name-resolution semantic fact only. |

#### Record `INV-VFS-009`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-009` |
| `Feature Name` | `keep_last_dots affects lookup equivalence but not trailing-dot creation refusal` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `lookup`, `create`, and rename-to-new-name validation for names ending in trailing dots. |
| `Primary State / Objects` | User-supplied terminal name component, trailing-dot normalization choice, and create/rename validation path. |
| `Required Invariant / Guarantee` | `keep_last_dots` can preserve trailing-dot spelling during lookup equivalence, but create and rename-to-new-name paths still reject names ending in `.` with `-EINVAL`. |
| `Failure / Edge Conditions` | A spelling that is legal to resolve under lookup semantics may still be illegal to create as a fresh target name. |
| `Mount / Policy Sensitivity` | Sensitive to `keep_last_dots`. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding` |
| `Ownership Notes` | Reserved. Keep this as a refusal-boundary fact, not as a design hint. |

#### Record `INV-VFS-010`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-010` |
| `Feature Name` | `Alias-bearing names require identity reuse, but case-only rename coherence may stay partial` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Lookup/cache reconciliation when long-name and 8.3 alias spellings converge on one object, plus case-only rename scenarios for already-instantiated names. |
| `Primary State / Objects` | Multiple user-visible spellings for one on-disk object, existing instantiated identity/cache state, and positive-name coherence after case-only rename. |
| `Required Invariant / Guarantee` | When distinct spellings resolve to the same on-disk object, the namespace carrier must reuse or reconcile existing identity instead of instantiating duplicates. Purely case-only renames may still leave already-instantiated positive-name state requiring explicit coherence handling rather than being silently assumed correct. |
| `Failure / Edge Conditions` | Failing to reuse or reconcile aliases can duplicate identity for one object; failing to define case-only rename coherence leaves stale positive-name state as an acknowledged limitation or explicit repair obligation. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. Linux currently realizes alias reconciliation through dentry-alias reuse paths, but this row records only the identity-coherence requirement and limitation. |

### Cluster C. Allocation / Size Mutation / Write Ordering

#### Record `INV-VFS-011`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-011` |
| `Feature Name` | `Fallocate-family requests are currently refused rather than remapped into ordinary writes` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Explicit preallocation and range-manipulation requests, including preallocation, zero-range, hole-punch, collapse-range, and insert-range variants. |
| `Primary State / Objects` | Requested fallocate mode or operation family, target file-size/allocation mutation interfaces, and the explicit refusal result. |
| `Required Invariant / Guarantee` | The currently authorized exFAT semantics refuse the fallocate family rather than translating those requests into ordinary write, truncate, or synthetic zero-fill behavior. |
| `Failure / Edge Conditions` | Any implementation that mirrors the current prior semantics must refuse unsupported fallocate-family requests explicitly; it must not silently invent a backing allocation behavior just because the target VFS exposes a fallocate carrier. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`, `2.9 Unsupported VFS Features`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. Linux currently surfaces this through the absence of a `fallocate` carrier, while Asterinas exposes `Inode::fallocate`; this row records the capability/refusal boundary rather than either carrier shape. |

#### Record `INV-VFS-012`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-012` |
| `Feature Name` | `Non-contiguous growth flips NoFatChain and backfills FAT links` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | File expansion that allocates a new cluster not physically contiguous with the current contiguous stream. |
| `Primary State / Objects` | In-memory chain mode (`ALLOC_NO_FAT_CHAIN` versus `ALLOC_FAT_CHAIN`), newly allocated cluster, previously contiguous run, allocation bitmap, FAT entries, and Stream Extension `NoFatChain` / `DataLength` fields. |
| `Required Invariant / Guarantee` | When growth discovers the first non-contiguous cluster after a contiguous run, the stream must stop being treated as `NoFatChain`: the implementation flips into FAT-chain mode, backfills FAT links for the previously arithmetic-only run, marks the new allocation in the bitmap, and then syncs the new chain mode and sizes into the directory entry. |
| `Failure / Edge Conditions` | Leaving the stream marked contiguous after a discontinuous allocation would misdescribe the on-disk mapping and break later resolution of cluster locations. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`, `1.4 Page Cache Block Translation`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 7.6.2.2 NoFatChain Field`, `#### 7.6.6 FirstCluster Field`, `#### 7.6.7 DataLength Field` |
| `Ownership Notes` | Reserved. Record the state transition only; do not infer the eventual owner boundary from it. |

#### Record `INV-VFS-013`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-013` |
| `Feature Name` | `Writes must zero-fill the gap between valid_size and new write exposure` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Buffered writes or mmap write-faults that begin beyond the current initialized-data extent. |
| `Primary State / Objects` | Visible EOF (`i_size`), initialized extent (`valid_size` / `ValidDataLength`), address-space `write_begin` / `write_end`, and zero-fill of newly exposed buffers. |
| `Required Invariant / Guarantee` | If a write reaches beyond the current initialized-data extent, Linux exFAT must extend `valid_size` through zero-fill before exposing the new range, so readers never observe uninitialized media contents inside the logical file size. |
| `Failure / Edge Conditions` | Skipping the zero-fill step would expose undefined bytes; mmap write-faults must obey the same invariant rather than bypassing it. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`, `2.6 Runtime File I/O Surface`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.6.5 ValidDataLength Field`, `#### 7.6.7 DataLength Field` |
| `Ownership Notes` | Reserved. Keep this as an initialized-data guarantee, not as an implementation sketch. |

#### Record `INV-VFS-014`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-014` |
| `Feature Name` | `Append growth acknowledges new EOF only after allocation state is durable enough` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `O_APPEND` writes and later inode writeback that persists new stream lengths. |
| `Primary State / Objects` | `i_size`, `valid_size`, allocation bitmap, FAT chain state, and directory-entry writeback ordering in `__exfat_write_inode`. |
| `Required Invariant / Guarantee` | For append-style growth, exFAT syncs the directory entry last, after the bitmap/FAT state needed for the new extent is established. If a crash interrupts the sequence, the old visible size may survive and newly allocated clusters may become orphans, but uninitialized space must not become part of the acknowledged file length. |
| `Failure / Edge Conditions` | Crash windows may leak clusters, but must not publish a larger EOF before the underlying cluster mapping is in place. |
| `Mount / Policy Sensitivity` | Sensitive to append semantics (`O_APPEND`), not to a mount option. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 8.1 Recommended Write Ordering` |
| `Ownership Notes` | Reserved. This is a write-ordering guarantee only. |

#### Record `INV-VFS-015`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-015` |
| `Feature Name` | `truncate and setattr shrink route through explicit chain-shortening semantics` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `setattr` / truncate-style size decreases and any explicit shrink of an existing data stream. |
| `Primary State / Objects` | `ATTR_SIZE` handling, `exfat_setattr()`, `exfat_truncate()`, file size, stream `DataLength`, and the underlying FAT/bitmap state for released clusters. |
| `Required Invariant / Guarantee` | Size decreases are not handled by `fallocate`; they route through explicit truncate/setattr logic that shrinks the stream and updates the underlying allocation state to match the new smaller file size. |
| `Failure / Edge Conditions` | Treating shrink as a write-style growth path would leave released clusters and directory-entry size state inconsistent with each other. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. This is a size-decrease fact only and should not be read as a topology decision. |

#### Record `INV-VFS-016`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-016` |
| `Feature Name` | `Asterinas inode surface forces an explicit stance on resize, write_at flags, and fallocate variants` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Asterinas `Inode` operations `resize`, `write_at`, and `fallocate`, including `O_APPEND`, `O_DIRECT`, and explicit `FallocMode` variants. |
| `Primary State / Objects` | Asterinas `Inode` trait methods `resize`, `write_at`, `fallocate`, `StatusFlags`, and `FallocMode` variants. |
| `Required Invariant / Guarantee` | The Asterinas integration surface requires the filesystem to answer explicit resize and write-path semantics, and to take an explicit position on fallocate-family requests even if the chosen behavior is refusal. Direct-I/O style writes also require alignment checks at the VFS/inode boundary. |
| `Failure / Edge Conditions` | Any eventual exFAT implementation that mirrors Linux refusal semantics must still reject the unsupported `FallocMode` variants deliberately rather than leaving the trait surface semantically unspecified. |
| `Mount / Policy Sensitivity` | Sensitive to status flags such as `O_APPEND`, `O_SYNC`, `O_DSYNC`, and `O_DIRECT`; not tied to a mount option. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation`, `2.6 Runtime File I/O Surface` |
| `Ownership Notes` | Reserved. This is an integration-pressure fact, not a prescribed component split. |

### Cluster D. Directory Lifecycle / Tree Mutability

#### Record `INV-VFS-017`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-017` |
| `Feature Name` | `create and mkdir secure directory slots before committing new entry sets` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `create` and `mkdir` in a mutable parent directory. |
| `Primary State / Objects` | Parent directory entry stream, contiguous empty/deleted slots, new file or directory entry set, and any parent-directory cluster extension needed to host the new set. |
| `Required Invariant / Guarantee` | Creation first secures a large-enough contiguous slot range in the parent directory. If the directory is full and already has a real cluster chain, exFAT may extend the directory before writing the new entry set. Only after slots are secured does it materialize the new entry triplet/set. |
| `Failure / Edge Conditions` | A parent directory with insufficient free slots cannot accept a new entry set until space is found or the directory itself is extended. |
| `Mount / Policy Sensitivity` | `zero_size_dir` later affects initial directory allocation shape for `mkdir`, but not the need to secure parent-directory slots first. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity`, `2.8 Directory Creation Shape` |
| `Ownership Notes` | Reserved. Keep this as a creation-ordering fact only. |

#### Record `INV-VFS-018`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-018` |
| `Feature Name` | `unlink and rmdir invalidate entry sets before freeing their cluster state` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `unlink` and `rmdir` against existing directory entry sets. |
| `Primary State / Objects` | Target directory entry set, deleted entry marker (`TYPE_DELETED` / `0xE5` in Linux), parent directory writeback, allocation bitmap, and FAT chain for the removed object. |
| `Required Invariant / Guarantee` | Deletion first invalidates the target entry set on disk and only then frees the associated cluster state. This preserves the ordering boundary between namespace disappearance and cluster reclamation. |
| `Failure / Edge Conditions` | Freeing cluster state before the directory entry is invalidated would risk leaving live namespace references pointing at reclaimed data. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 6.2.1 EntryType Field`, `### 8.1 Recommended Write Ordering` |
| `Ownership Notes` | Reserved. This is a deletion-ordering fact only. |

#### Record `INV-VFS-019`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-019` |
| `Feature Name` | `Cross-directory rename secures the new home before invalidating the old one` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `rename` that moves an entry across directory locations, with or without overwrite. |
| `Primary State / Objects` | Source entry set, destination slot range, cloned destination entry set, old entry invalidation, and any overwritten target's eventual cluster reclamation. |
| `Required Invariant / Guarantee` | Cross-directory rename first secures destination slots, clones and updates the entry set into the new parent, writes the new directory view, and only then invalidates the old source entry set. If overwriting an existing target, its old clusters are reclaimed only after the replacement entry set is safely established. |
| `Failure / Edge Conditions` | Crash windows may temporarily leave two entry sets pointing at the same cluster chain, but the ordering avoids making the moved object completely unreachable. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity` |
| `Ownership Notes` | Reserved. Record the rename sequencing fact only. |

#### Record `INV-VFS-020`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-020` |
| `Feature Name` | `directory removal and directory-target rename require an emptiness gate` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `rmdir` and rename paths whose destination already exists as a directory. |
| `Primary State / Objects` | Candidate directory contents, emptiness check (`exfat_check_dir_empty` in Linux), and VFS-visible `ENOTEMPTY` refusal boundary. |
| `Required Invariant / Guarantee` | A directory may be removed only when it is empty, and a rename that would overwrite an existing directory target must first verify the target directory is empty before proceeding. |
| `Failure / Edge Conditions` | Non-empty directories are refused rather than partially overwritten or removed; this refusal boundary is part of normal tree-mutability semantics, not a recovery-only path. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces`, `6. Expected Error Variants (Errno)` |
| `Ownership Notes` | Reserved. This is a refusal-boundary fact only. |

#### Record `INV-VFS-021`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-021` |
| `Feature Name` | `rename accepts ordinary semantics and RENAME_NOREPLACE only` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | VFS rename requests carrying flag combinations. |
| `Primary State / Objects` | Rename flag set, VFS prechecks, and exFAT rename entry path. |
| `Required Invariant / Guarantee` | The current Linux exFAT prior accepts plain rename semantics plus the no-overwrite `RENAME_NOREPLACE` variant, and rejects other rename flags with `-EINVAL`. |
| `Failure / Edge Conditions` | Unsupported rename-flag combinations fail before exFAT tries to reinterpret them as another operation kind. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity` |
| `Ownership Notes` | Reserved. This is the Linux rename-ABI boundary only. |

#### Record `INV-VFS-022`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-022` |
| `Feature Name` | `zero_size_dir changes only the newborn directory's initial allocation shape` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `mkdir` under mounts with or without `zero_size_dir`. |
| `Primary State / Objects` | New directory start cluster, `size`, `valid_size`, and the mount option `zero_size_dir`. |
| `Required Invariant / Guarantee` | With `zero_size_dir` disabled, `mkdir` allocates the first directory cluster immediately. With it enabled, the new directory begins with `EXFAT_EOF_CLUSTER`, `size = 0`, and `valid_size = 0`, deferring physical directory allocation until later population. |
| `Failure / Edge Conditions` | The option changes the newborn directory's initial storage shape, but not the requirement that the namespace entry itself be created in the parent directory. |
| `Mount / Policy Sensitivity` | Fully sensitive to `zero_size_dir`. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.8 Directory Creation Shape` |
| `Ownership Notes` | Reserved. Keep this as a mount-sensitive creation fact, not as a remount rule. |

#### Record `INV-VFS-023`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-023` |
| `Feature Name` | `Asterinas path-tree surface is positional and still requires explicit refusal boundaries` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Asterinas `Inode` path-tree operations `create`, `unlink`, `rmdir`, `rename`, `link`, and symlink-related hooks. |
| `Primary State / Objects` | Asterinas `Inode` trait methods for namespace mutation, the local `rename` signature without passed-through rename flags, and Linux exFAT's unsupported-link and unsupported-symlink facts. |
| `Required Invariant / Guarantee` | The Asterinas integration surface requires explicit semantics for path-tree mutation and explicit refusals for unsupported capabilities. Its current `rename` surface is positional because rename flags are not passed through to `Inode::rename`. For an exFAT-like implementation, hardlink and symlink support still cannot remain implicit; they must resolve to deliberate `EPERM` / `EOPNOTSUPP`-style behavior rather than an unspecified gap. |
| `Failure / Edge Conditions` | Leaving unsupported tree operations semantically unspecified would create owner gaps at the VFS boundary even if the on-disk format lacks those features intrinsically. Separately, Linux-specific rename-flag behavior must not be over-read as if the current Asterinas rename hook already carries those flags. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.9 Unsupported VFS Features` |
| `Ownership Notes` | Reserved. This is an integration-boundary fact, not a pass-slicing hint. |

### Cluster E. Page-Cache / Block Mapping / Runtime I/O

#### Record `INV-VFS-024`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-024` |
| `Feature Name` | `Block mapping takes an O(1) path for contiguous NoFatChain streams` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `bmap` / `get_block` resolving a logical file block into backing storage sectors. |
| `Primary State / Objects` | Logical block index, stream `NoFatChain` state, `start_clu`, FAT chain traversal state, and mapped backing sectors/clusters. |
| `Required Invariant / Guarantee` | When a stream is still marked `NoFatChain`, logical-block translation may compute the mapped cluster arithmetically from the starting cluster and offset. Once the stream is FAT-chained, mapping must walk the FAT instead of pretending arithmetic contiguity still holds. |
| `Failure / Edge Conditions` | Treating a FAT-chained stream as still contiguous would direct reads and writes to the wrong sectors. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.4 Page Cache Block Translation`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 7.6.2.2 NoFatChain Field`, `#### 7.6.6 FirstCluster Field` |
| `Ownership Notes` | Reserved. This is a mapping behavior fact only. |

#### Record `INV-VFS-028`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-028` |
| `Feature Name` | `Block mapping and size mutation require a synchronization boundary against truncate` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `bmap` / block-mapping reads racing against truncate or extend paths that may free or rewire clusters. |
| `Primary State / Objects` | Block-mapping path, truncate/extend path, file-size mutation state, and whatever synchronization boundary keeps mapping and size mutation from observing inconsistent cluster relationships. |
| `Required Invariant / Guarantee` | Logical block mapping must not race against concurrent truncate or extend work that may free or rewrite the same cluster relationships. The inventory records only the need for a read/write anti-race boundary between mapping and size mutation; the exact target-side lock or sequencing carrier remains open. |
| `Failure / Edge Conditions` | Without a synchronization boundary, block mapping could resolve sectors that are simultaneously being truncated away or remapped. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.4 Page Cache Block Translation`, `1.5 Concurrency & Locking Topology` |
| `Ownership Notes` | Reserved. Linux currently realizes this with `truncate_lock`, but this row records only the anti-race boundary, not a prescribed lock shape. |

#### Record `INV-VFS-025`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-025` |
| `Feature Name` | `fsync spans file writeback, block-device sync, and device-cache flush` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `fsync` / file-data sync requests on ordinary files. |
| `Primary State / Objects` | Dirty page-cache state, generic metadata writeback path, backing block device sync, and the final hardware/device flush. |
| `Required Invariant / Guarantee` | exFAT-style `fsync` must push dirty file state through the file-writeback path, synchronize the backing block device state, and request a final device-cache flush so volatile device-side data is also driven to persistence. |
| `Failure / Edge Conditions` | A forced-shutdown filesystem fails the sync path with `-EIO` instead of continuing into ordinary flush work. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.5 Data Sync` |
| `Ownership Notes` | Reserved. This is a sync-semantics fact only. |

#### Record `INV-VFS-026`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-026` |
| `Feature Name` | `Direct I/O must pass alignment gates before data movement begins` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Write-path requests carrying direct-I/O intent. |
| `Primary State / Objects` | File offset, data-buffer alignment state, filesystem/block geometry, backing-device alignment requirements, and the direct-I/O status flag. |
| `Required Invariant / Guarantee` | Direct-I/O style writes must satisfy the filesystem and backing-device alignment requirements before any allocation, zero-fill, or data-copy work begins. |
| `Failure / Edge Conditions` | Misaligned direct writes fail early with `-EINVAL` instead of degrading into a partially executed write path. |
| `Mount / Policy Sensitivity` | Sensitive to direct-I/O status flags, not to a mount option. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.6 Runtime File I/O Surface`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. This is an I/O gate fact, not an implementation prescription. |

#### Record `INV-VFS-027`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-027` |
| `Feature Name` | `Ordinary cached I/O uses generic cached-data paths with exFAT-specific boundary guards` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Ordinary cached read/write paths, mapping/page-fault preparation, splice-like transfer, and analogous target file-data entry points. |
| `Primary State / Objects` | Generic cached-data or page-cache machinery, exFAT-specific valid-size / write-fault guards, forced-shutdown state, and the target VFS file-data carriers. |
| `Required Invariant / Guarantee` | The current priors place ordinary cached I/O on generic cached-data or page-cache paths, with exFAT-specific logic concentrated at boundaries such as valid-size extension, page-cache preparation, and shutdown gating. The concrete carrier names may differ across VFSs, but the semantic shape remains cached-I/O reuse plus boundary-specific exFAT guards. |
| `Failure / Edge Conditions` | Forced-shutdown state must short-circuit ordinary data and sync entry paths with `EIO`-style failure instead of letting generic helpers continue against a shut-down volume. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.6 Runtime File I/O Surface`, `2.5 Data Sync`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `3. Exhaustive PageCacheBackend Interface`, `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. Linux currently realizes this through file-operations/filemap helpers, while Asterinas exposes different carriers; this row records the shared semantic shape only. |

### Cluster F. Permissions / Ownership / Timestamp / Timezone

#### Record `INV-VFS-029`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-029` |
| `Feature Name` | `Ownership and mode are mount-derived rather than natively persisted as POSIX metadata` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Inode metadata exposure, `chmod`, and `chown`-style setattr requests. |
| `Primary State / Objects` | Mount-level `fs_uid`, `fs_gid`, `dmask`, `fmask`, exposed inode mode/owner/group state, and incoming setattr requests. |
| `Required Invariant / Guarantee` | exFAT does not carry native UID/GID or POSIX mode bits on disk. Ownership and most permission semantics are synthesized from mount policy, and `chown` to values outside the mounted ownership envelope is rejected with `-EPERM`. |
| `Failure / Edge Conditions` | Treating ownership as natively persistent would overstate what the on-disk format guarantees; unsupported ownership changes are refused instead of silently inventing durable POSIX metadata. |
| `Mount / Policy Sensitivity` | Fully sensitive to `fs_uid`, `fs_gid`, `dmask`, and `fmask`. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.3 Permissions & Ownership Mapping` |
| `Ownership Notes` | Reserved. This is a mount-policy metadata fact only. |

#### Record `INV-VFS-030`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-030` |
| `Feature Name` | `chmod only materially propagates the writable bit through the DOS ReadOnly attribute` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `chmod` / mode-changing setattr requests and inode construction from on-disk attributes. |
| `Primary State / Objects` | VFS mode bits, `ATTR_RO` / DOS `ReadOnly` attribute, and sanitize/filter logic applied during mode updates. |
| `Required Invariant / Guarantee` | Writable-versus-read-only state is the only materially propagated POSIX-style mode effect: clearing or setting write permission toggles the DOS `ReadOnly` attribute, while execute semantics remain cosmetic and mount-mask-derived rather than durably enforced by exFAT metadata. |
| `Failure / Edge Conditions` | Overreading execute bits as durable exFAT permissions would misrepresent the actual metadata model; `ATTR_RO` also feeds back into rebuilt inode mode on lookup/build. |
| `Mount / Policy Sensitivity` | Sensitive to the mount masks that synthesize visible permission bits. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.3 Permissions & Ownership Mapping`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.4.4 FileAttributes Field` |
| `Ownership Notes` | Reserved. This is an attribute-mapping fact only. |

#### Record `INV-VFS-031`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-031` |
| `Feature Name` | `Timestamp translation bridges UTC-facing VFS state to exFAT local-time fields with offset bytes` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Reading or flushing inode timestamps between VFS-visible time values and exFAT on-disk timestamp fields. |
| `Primary State / Objects` | VFS timestamps, mount timezone policy (`tz_utc` / `tz_offset` logic), exFAT local-time timestamp fields, `UtcOffset` bytes, and 10ms increment fields. |
| `Required Invariant / Guarantee` | VFS-facing timestamp state must be translated against the mount's timezone policy when encoded into exFAT's local-time timestamp format. Create/modify timestamps also carry 10ms increment fields, while the UTC-offset side bytes describe the local-time offset from UTC. |
| `Failure / Edge Conditions` | Assuming exFAT timestamps are raw UTC or full-resolution POSIX times would misencode the stored metadata and lose the intended offset semantics. |
| `Mount / Policy Sensitivity` | Sensitive to timezone policy such as `tz_utc` / `tz_offset`. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.4 Timestamp & Timezone Translation`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.4.5 CreateTimestamp, Create10msIncrement, and CreateUtcOffset Fields`, `#### 7.4.6 LastModifiedTimestamp, LastModified10msIncrement, and LastModifiedUtcOffset Fields`, `#### 7.4.7 LastAccessedTimestamp and LastAccessedUtcOffset Fields`, `#### 7.4.10 UtcOffset Fields` |
| `Ownership Notes` | Reserved. This is a timestamp-translation fact only. |

#### Record `INV-VFS-033`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-033` |
| `Feature Name` | `Timestamp mutation is also gated by mount-time allow_utime policy` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Timestamp-changing setattr paths such as `set_atime` / `set_mtime` and any VFS metadata update logic subject to permission policy. |
| `Primary State / Objects` | Mount option `allow_utime`, synthesized permission policy, and timestamp setter/update requests. |
| `Required Invariant / Guarantee` | Timestamp updates are not only a question of on-disk encoding; they are also constrained by mount-time policy. Linux exFAT's metadata-permission envelope includes `allow_utime`, so timestamp mutation rights are partly mount-derived rather than purely inherent to the inode. |
| `Failure / Edge Conditions` | Ignoring the mount-level timestamp policy would overstate who may legally drive atime/mtime changes through the VFS surface. |
| `Mount / Policy Sensitivity` | Fully sensitive to `allow_utime` and the surrounding synthesized permission policy. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.3 Permissions & Ownership Mapping`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` |
| `Ownership Notes` | Reserved. This is a timestamp-policy fact only. |

#### Record `INV-VFS-032`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-032` |
| `Feature Name` | `Asterinas metadata surface requires explicit owner, mode, and timestamp answers even when exFAT synthesizes them` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Asterinas metadata getters/setters such as `metadata`, `mode`, `set_mode`, `owner`, `set_owner`, `group`, `set_group`, and timestamp accessors/mutators. |
| `Primary State / Objects` | Asterinas `Inode` metadata methods, synthesized owner/mode state, and timestamp getter/setter hooks. |
| `Required Invariant / Guarantee` | The target VFS expects explicit answers for owner, mode, group, size, blocks, and timestamps even if exFAT only stores a thinner DOS-style attribute/time model on disk. Any exFAT implementation on Asterinas must therefore map these metadata hooks deliberately instead of leaving them semantically implicit. In particular, the VFS exposes `ctime` / `set_ctime`, while the currently authorized exFAT priors only give native create / last-modified / last-accessed on-disk timestamp families. |
| `Failure / Edge Conditions` | Leaving owner/mode/timestamp hooks underspecified would create metadata owner gaps at the VFS boundary, especially for setters that the filesystem may need to reject or partially emulate. Overreading exFAT's native time fields as if they already provided a one-to-one on-disk `ctime` counterpart would also misstate the metadata model. |
| `Mount / Policy Sensitivity` | Sensitive to the mount-derived metadata policy that synthesizes visible ownership and permission state. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.3 Permissions & Ownership Mapping`, `2.4 Timestamp & Timezone Translation` |
| `Ownership Notes` | Reserved. This is an integration-boundary fact, not a prescribed owner split. |

### Cluster G. Administrative ABI / Unsupported / Refusal Boundary

#### Record `INV-VFS-034`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-034` |
| `Feature Name` | `DOS attribute ioctls expose only the legal exFAT attribute subset` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Administrative ioctl calls that get or set DOS/exFAT file attributes. |
| `Primary State / Objects` | DOS/exFAT attribute mask, directory bit preservation, root-directory special case, and mode-changing/setattr path reuse. |
| `Required Invariant / Guarantee` | Administrative attribute ABI does not expose arbitrary POSIX metadata writes. `GET_ATTRIBUTES` returns the synthesized DOS/exFAT mask, while `SET_ATTRIBUTES` masks userspace input down to legal exFAT DOS bits, preserves required directory semantics, and routes the writable-bit effect back through the normal setattr/mode-changing engine. |
| `Failure / Edge Conditions` | Invalid or unsupported attribute combinations are sanitized/rejected instead of being written as unconstrained metadata. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.4.4 FileAttributes Field` |
| `Ownership Notes` | Reserved. This is an administrative attribute-ABI fact only. |

#### Record `INV-VFS-035`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-035` |
| `Feature Name` | `FITRIM and online discard are distinct administrative free-space paths` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | `FITRIM` ioctl calls, mount-time `discard`, and runtime free-cluster release. |
| `Primary State / Objects` | Allocation bitmap scans, discard-capable backing device state, `minlen` / discard granularity, contiguous free-cluster runs, and the runtime online-discard enable bit. |
| `Required Invariant / Guarantee` | `FITRIM` is an administrative bulk-trim path over currently free ranges, while mount-time `discard` is an opportunistic runtime hint on future frees. They are related but independent: each has its own gating checks and failure behavior. |
| `Failure / Edge Conditions` | `FITRIM` requires administrative privilege and discard-capable media; online discard can be disabled independently at mount or runtime without disabling the filesystem's ability to free clusters correctly. |
| `Mount / Policy Sensitivity` | Sensitive to the `discard` mount option and backing-device discard capability. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`, `2.1 Initialization & Global Status` |
| `Ownership Notes` | Reserved. This is an administrative free-space ABI fact only. |

#### Record `INV-VFS-036`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-036` |
| `Feature Name` | `Forced shutdown is an administrative state transition with follow-on runtime effects` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Administrative shutdown ioctl requests and later ordinary I/O/free-space paths after shutdown is active. |
| `Primary State / Objects` | Shutdown ioctl flags, forced-shutdown bit, optional freeze/thaw path, runtime discard enablement, and later I/O entry paths that observe shutdown state. |
| `Required Invariant / Guarantee` | Administrative forced shutdown is a first-class state transition, not just an error return. Once active, later ordinary I/O surfaces fail fast with `-EIO`. The current Linux prior also suppresses later online discard so free-cluster paths stop issuing discard requests. |
| `Failure / Edge Conditions` | Shutdown control itself requires administrative privilege, and later data-path calls must not continue as if the filesystem were still in a normal mutable state. |
| `Mount / Policy Sensitivity` | Sensitive to the runtime `discard` state because shutdown suppresses it. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`, `2.6 Runtime File I/O Surface` |
| `Ownership Notes` | Reserved. This is a global-state/administrative ABI fact only. |

#### Record `INV-VFS-037`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-037` |
| `Feature Name` | `Volume label updates are privileged, encoding-aware, and refusal-prone on lossy conversion` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Filesystem-label get/set ioctls. |
| `Primary State / Objects` | On-disk volume-label dentry, active user-facing encoding, UTF-16 conversion path, and privilege check for label mutation. |
| `Required Invariant / Guarantee` | Getting a filesystem label reads and converts the on-disk volume label into the active user-facing encoding. Setting a label is privileged, must convert the supplied label into exFAT's UTF-16 representation, and must reject lossy or invalid conversion rather than silently mangling the stored administrative label. |
| `Failure / Edge Conditions` | Lossy label conversion fails with `-EINVAL`, and label mutation is denied to non-privileged callers. |
| `Mount / Policy Sensitivity` | Sensitive to the active user-facing encoding configuration. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 7.3.3 VolumeLabel Field` |
| `Ownership Notes` | Reserved. This is a volume-label administrative ABI fact only. |

#### Record `INV-VFS-038`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-038` |
| `Feature Name` | `Refusal paths must preserve typed errno distinctions instead of collapsing into implicit gaps` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | VFS calls that are refused because the capability is unsupported, the arguments are invalid, or the volume is read-only. |
| `Primary State / Objects` | Refusal reason class, typed errno result (`EOPNOTSUPP`, `EPERM`, `EINVAL`, `EROFS`), and Asterinas' explicit ban on returning `ENOSYS` from filesystem logic. |
| `Required Invariant / Guarantee` | Refusal paths must stay typed: unsupported capabilities use the unsupported-operation family, invalid requests use argument errors, and read-only rejections use the read-only family instead of collapsing everything into one generic failure. In the Asterinas target, these refusals must not collapse into `ENOSYS`. |
| `Failure / Edge Conditions` | Over-broad or missing errno mapping would blur the difference between unsupported operations, invalid arguments, and read-only rejections. |
| `Mount / Policy Sensitivity` | Some refusal outcomes are mount- or flag-sensitive, such as `EROFS` on read-only volumes, but the typed-refusal rule itself is not a mount option. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`, `2.9 Unsupported VFS Features`, `1.2 Tree Mutability & Atomicity`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `6. Expected Error Variants (Errno)` |
| `Ownership Notes` | Reserved. This is a refusal-boundary fact only. |

#### Record `INV-VFS-039`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-039` |
| `Feature Name` | `Administrative Linux ABIs still require an explicit Asterinas carrier or deliberate refusal surface` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Management operations such as DOS-attribute ioctls, `FITRIM`, forced shutdown, and filesystem-label get/set. |
| `Primary State / Objects` | Linux-side ioctl-style administrative ABI semantics, current Asterinas VFS/inode trait surface, and any future adapter or refusal boundary used to expose or reject those operations. |
| `Required Invariant / Guarantee` | The current priors define the Linux semantics of these management operations, but do not yet provide a one-to-one Asterinas carrier surface for all of them. An exFAT implementation on Asterinas must therefore make each administrative ABI either explicitly carried by a local interface or explicitly refused through a typed boundary, rather than assuming an existing hook already exists. |
| `Failure / Edge Conditions` | Assuming a Linux ioctl-shaped administrative feature already has a native target-side home would create owner gaps in later architecture mapping. |
| `Mount / Policy Sensitivity` | Some administrative ABIs are mount- or runtime-state-sensitive, but the carrier-gap fact itself is not a mount option. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces`, `6. Expected Error Variants (Errno)` |
| `Ownership Notes` | Reserved. This is an integration-pressure fact only, not a topology decision. |

### Cluster H. Consistency / Recovery / Anomaly Surface

#### Record `INV-VFS-040`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-040` |
| `Feature Name` | `Append crash windows prefer orphaned allocation over exposing uninitialized logical EOF` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Append-style growth and later inode writeback that persists the enlarged stream. |
| `Primary State / Objects` | `i_size`, `valid_size`, allocation bitmap/FAT state, and the final directory-entry acknowledgment of new EOF. |
| `Required Invariant / Guarantee` | When append growth is interrupted mid-sequence, Linux exFAT prefers leaving newly allocated clusters orphaned over publishing a larger logical size that could expose uninitialized contents. |
| `Failure / Edge Conditions` | Crash recovery may encounter leaked/orphaned clusters, but the visible file length should remain at the older acknowledged size instead of exposing partially initialized new space. |
| `Mount / Policy Sensitivity` | Sensitive to append-style writes, not to a mount option. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.1 Allocation & Size Mutation` |
| `Ownership Notes` | Reserved. This is a crash-window fact only. |

#### Record `INV-VFS-041`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-041` |
| `Feature Name` | `Cross-directory rename crash windows may transiently duplicate reachability rather than orphan it` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Cross-directory rename, especially when the new entry is written before the old one is invalidated. |
| `Primary State / Objects` | Old and new directory entry sets, target cluster chain, and the sequencing of new-parent versus old-parent entry writes. |
| `Required Invariant / Guarantee` | The rename sequence is chosen so that a crash can transiently leave two directory entries pointing at the same cluster chain, but avoids making the moved object completely unreachable. |
| `Failure / Edge Conditions` | Recovery tooling may need to reconcile duplicate reachability after a crash rather than just reclaim a lost object. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.2 Tree Mutability & Atomicity` |
| `Ownership Notes` | Reserved. This is a crash-window fact only. |

#### Record `INV-VFS-042`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-042` |
| `Feature Name` | `Mount-time accounting may fall back to recount under corruption-recovery conditions` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Mount-time or runtime free-space reporting when incremental `used_clusters` accounting can no longer be trusted. |
| `Primary State / Objects` | `used_clusters`, allocation bitmap scan, `statfs` reporting path, and corruption-recovery edge conditions. |
| `Required Invariant / Guarantee` | Normal free-space reporting uses incrementally maintained `used_clusters`, but corruption-recovery situations may force a recount from the allocation bitmap instead of trusting the cached accounting state. |
| `Failure / Edge Conditions` | Assuming cached accounting remains trustworthy in corruption-recovery scenarios would misstate free-space and recovery posture. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status` |
| `Ownership Notes` | Reserved. This is a recovery-accounting fact only. |

#### Record `INV-VFS-043`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-043` |
| `Feature Name` | `Dirty, media-failure, clear-to-zero, and forced-shutdown states are anomaly surfaces, not ordinary steady-state paths` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Mounting a volume with anomaly bits set, administrative forced shutdown, or data paths that observe shutdown/error state. |
| `Primary State / Objects` | `VolumeDirty`, `MediaFailure`, `ClearToZero`, forced-shutdown state, and ordinary read/write/fsync/splice/mmap entry points that must interpret those states. |
| `Required Invariant / Guarantee` | These states signal anomaly or recovery-sensitive conditions rather than ordinary steady-state operation. Forced shutdown converts later data paths into fast-fail surfaces, dirty/media-failure signal post-mount caution or recovery posture, and `ClearToZero = 1` means an implementation must clear that bit before modifying filesystem structures, directories, or files. |
| `Failure / Edge Conditions` | Treating anomaly flags as if the filesystem were in a normal mutable state would blur recovery-sensitive behavior, miss the `ClearToZero` pre-modification requirement, and weaken later error handling. |
| `Mount / Policy Sensitivity` | Not a mount option, though later behavior may interact with runtime `discard` state after forced shutdown. |
| `Primary Source Anchors` | `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.6 Runtime File I/O Surface`, `2.7 Administrative & Maintenance ABI`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.2 VolumeDirty Field`, `##### 3.1.13.3 MediaFailure Field`, `##### 3.1.13.4 ClearToZero Field` |
| `Ownership Notes` | Reserved. This is an anomaly-surface fact only. |

#### Record `INV-VFS-044`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-VFS-044` |
| `Feature Name` | `Unrecognized directory entries impose typed invalidity and no-modify boundaries instead of generic ignore behavior` |
| `Layer` | `VFS/Interface` |
| `Trigger / Entry Surface` | Mounting or scanning directories with unrecognized directory entries, plus delete/traverse operations that encounter them. |
| `Primary State / Objects` | Unrecognized critical primary, benign primary, critical secondary, and benign secondary directory entries; their containing directory or directory-entry set; and any associated cluster allocations. |
| `Required Invariant / Guarantee` | Unrecognized directory entries do not collapse into one generic policy. Unrecognized critical primaries invalidate the volume root or hosting non-root directory; unrecognized critical secondaries make the whole directory-entry set unrecognized and forbid ordinary modification/open of that set; unrecognized benign entries are not free-form mutation targets either, though deletion paths still carry explicit cluster-freeing obligations. |
| `Failure / Edge Conditions` | Treating all unknown entries as either harmlessly ignorable or universally fatal would lose the spec's typed anomaly boundaries. In particular, directory sets containing unrecognized critical secondaries still have limited directory-only allowances such as traverse, enumerate, delete-contained-entry, and move-contained-entry. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 8.2 Implications of Unrecognized Directory Entries` |
| `Ownership Notes` | Reserved. This is an anomaly-surface fact only. |

## 4. BIO Substrate Layer

This section is reserved for source-backed micro-features about page/block geometry, blocking boundaries, writeback, flush behavior, and storage-facing constraints.
Populate it incrementally from the Linux verification notes and Asterinas integration priors.

### Cluster E. Page-Cache / Block Mapping / Runtime I/O

#### Record `INV-BIO-001`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-BIO-001` |
| `Feature Name` | `PageCacheBackend must translate page indices into possibly fragmented disk I/O without blocking inline` |
| `Layer` | `BIO Substrate` |
| `Trigger / Entry Surface` | `PageCacheBackend::read_page_async`, `write_page_async`, and `npages` for cached file I/O. |
| `Primary State / Objects` | Page-cache page index, `CachePage`, fragmented cluster/block mapping, BIO construction, and returned `BioWaiter`. |
| `Required Invariant / Guarantee` | A page-cache backend must map each page index to the correct on-disk sectors even when a single page spans disjoint backing extents. The async page-cache hooks must return a `BioWaiter` instead of blocking inline for completion. |
| `Failure / Edge Conditions` | Assuming a page is physically contiguous when the file is fragmented would misdirect I/O; blocking inline would violate the async page-cache contract. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `3. Exhaustive PageCacheBackend Interface`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `1.4 Page Cache Block Translation` |
| `Ownership Notes` | Reserved. This is a substrate mapping fact only. |

#### Record `INV-BIO-002`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-BIO-002` |
| `Feature Name` | `BlockDevice I/O completion may suspend the thread and force post-wakeup state revalidation` |
| `Layer` | `BIO Substrate` |
| `Trigger / Entry Surface` | Synchronous block methods, async block submission returning `BioWaiter`, and later `BioWaiter::wait()` completion. |
| `Primary State / Objects` | `BlockDevice` sync/async I/O entry points, `BioWaiter`, suspended thread state, and any shared filesystem state that may need revalidation after wakeup. |
| `Required Invariant / Guarantee` | Submitting block I/O or waiting on a `BioWaiter` can suspend the current thread. Any filesystem state that was only conditionally trusted before the sleep boundary may need to be revalidated after wakeup rather than assumed unchanged across the wait. |
| `Failure / Edge Conditions` | Treating a sleep boundary as if no concurrent state change were possible risks using stale mapping or metadata assumptions after I/O completion. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `2. Exhaustive BIO (Block I/O) Interfaces` |
| `Ownership Notes` | Reserved. This is a blocking-boundary fact only. |

#### Record `INV-BIO-003`

| Field | Value |
| :--- | :--- |
| `Feature ID` | `INV-BIO-003` |
| `Feature Name` | `SpinLock critical sections cannot enclose block I/O or BioWaiter waits` |
| `Layer` | `BIO Substrate` |
| `Trigger / Entry Surface` | Any runtime-I/O or page-cache path that might submit block I/O, wait on a `BioWaiter`, or otherwise cross a yielding/blocking boundary while holding a lock. |
| `Primary State / Objects` | `SpinLock`, block-I/O submission, `BioWaiter::wait()`, and any yielding lock acquisition attempted inside the same critical section. |
| `Required Invariant / Guarantee` | Asterinas `SpinLock` critical sections must not perform block I/O, wait on a `BioWaiter`, or acquire yielding locks. These I/O boundaries imply potential suspension and therefore are incompatible with preemption-disabled spin-locked sections. |
| `Failure / Edge Conditions` | Violating this boundary can deadlock or panic the kernel instead of merely slowing the path down. |
| `Mount / Policy Sensitivity` | Not mount-option-driven. |
| `Primary Source Anchors` | `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `1. Asterinas Lock Primitives & Concurrency Substrate`, `2. Exhaustive BIO (Block I/O) Interfaces` |
| `Ownership Notes` | Reserved. This is a substrate safety fact only. |

## 5. Fill Discipline

When adding records later:

1. Add only source-backed rows.
2. Do not imply macro-owner assignment, meso boundaries, or Creator/Checker pass slicing here.
3. Prefer leaving a gap over inserting a guessed feature row.
4. Keep cross-layer relationships factual; do not turn them into architecture decisions inside this file.

## 6. Cross-Cutting Reading Guide

This chapter does not introduce new feature rows.
It exists only to help Architect and later readers locate already-recorded facts that form recurring cross-cutting themes.

### 6.1 Concurrency / Lock-Boundary / Sleep-Boundary Guide

This is not a standalone cluster.
The relevant facts already live in the inventory rows below and should be read together when reasoning about synchronization, blocking, and race boundaries:

| Theme | Existing Rows | Why They Belong Together |
| :--- | :--- | :--- |
| Mapping versus size mutation anti-race | `INV-VFS-028` | Captures the requirement that block mapping must not observe truncate/extend in an inconsistent intermediate state. |
| Directory/tree mutation ordering and overwrite gates | `INV-VFS-018`, `INV-VFS-019`, `INV-VFS-020` | Capture deletion/rename ordering plus overwrite and emptiness refusal boundaries that later synchronization decisions must preserve. |
| Cached-data path and mapping boundaries around mutable stream state | `INV-VFS-024`, `INV-VFS-027`, `INV-VFS-028` | Capture logical mapping behavior, generic cached-I/O reuse, and the anti-race boundary against concurrent size mutation. |
| File-data sync semantics across generic writeback and device flush | `INV-VFS-025` | Captures the separate sync path that must still compose correctly with the surrounding cached-data and block-device boundaries. |
| BIO sleep boundary and post-wakeup revalidation | `INV-BIO-002` | Captures that block I/O completion may suspend the thread and therefore invalidate pre-sleep assumptions. |
| Spinlock incompatibility with I/O and waits | `INV-BIO-003` | Captures the hard Asterinas substrate rule that blocking/yielding I/O boundaries cannot live inside `SpinLock` critical sections. |

### 6.2 Carrier-Mismatch / Target-Surface-Pressure Guide

This is also not a standalone cluster.
The relevant facts already live in the inventory rows below and should be read together when translating Linux-observed semantics onto Asterinas surfaces:

| Theme | Existing Rows | Why They Belong Together |
| :--- | :--- | :--- |
| Resize/write/fallocate pressure from the Asterinas inode surface | `INV-VFS-016` | Captures that Asterinas exposes explicit resize, write, and fallocate carriers even when exFAT semantics may refuse some operation families. |
| Namespace mutation and unsupported tree-operation pressure | `INV-VFS-023` | Captures the positional Asterinas rename surface and the need for explicit refusal boundaries for unsupported tree capabilities. |
| Metadata-surface pressure beyond native exFAT metadata width | `INV-VFS-032` | Captures that Asterinas expects explicit metadata answers, including hooks like `ctime`, even when exFAT only persists a thinner on-disk model. |
| Administrative ABI carrier gap | `INV-VFS-039` | Captures that Linux administrative ABIs such as ioctl-shaped management operations do not automatically have one-to-one Asterinas carriers and must be carried or refused deliberately. |

Read this chapter as a navigation aid only.
If a new cross-cutting concern is not already supported by concrete source-backed rows elsewhere in the inventory, add the missing rows first instead of extending this guide with unsupported synthesis.
