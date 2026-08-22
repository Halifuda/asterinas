<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlayfs 符号-文件映射表（Symbol-File Map）

> 状态：初步整理，用于代码结构重构时的搬移参考。
> 范围：当前 active overlayfs 代码（排除 `legacy_fs.rs` 与 proposal 已明确删除/消灭的类型）。
> 方法：从当前源码提取 struct/enum/重要 fn，按预期目标文件结构归类。简单 wrapper/accessor/trait 转发不列。
> 注意：本表是“目标结构下的符号归属草案”，不是最终裁决；后续可根据依赖关系再调整。

## 目标文件结构（采用当前 proposal + 已讨论的调整）

```text
overlayfs/
├── mod.rs
├── fs_type.rs
├── layer.rs          # Layer / LayerStack / RealObjectStack
├── real.rs           # RealObject / RealPath / RealObjectKey
├── fs/
│   ├── mod.rs
│   ├── policy.rs
│   └── mount/
│       ├── mod.rs
│       ├── options.rs
│       ├── layer_parts.rs
│       ├── inuse.rs
│       └── capabilities.rs
└── inode/
    ├── mod.rs
    ├── inode_cache.rs
    ├── lookup.rs
    ├── identity.rs
    ├── readdir.rs
    ├── copyup/
    │   ├── mod.rs
    │   └── workdir.rs
    ├── dir/
    │   ├── mod.rs
    │   ├── create.rs / link.rs / remove.rs / rename.rs
    │   └── whiteout.rs
    ├── data.rs
    ├── permission.rs
    ├── metadata.rs
    └── xattr.rs
```

已讨论的调整：

- `real/` 不再保留目录，改为顶层单文件 `real.rs`。
- `layer.rs` 提升为顶层单文件，包含 `Layer` / `LayerStack` / `RealObjectStack`。
- 不设 `real/records.rs`；xattr marker 与记录读写归 `inode/xattr.rs`。
- `RealObjectKey` 从旧 `inode/lookup/key.rs` 移到 `real.rs`。
- `RealObjectStack` 表示一个 overlay 对象背后的真实对象组合（`upper` + `lowers`）；`PositiveKind` 改为派生或保留在 inode 侧。
- `lookup/key.rs` 不再单独存在；`RealObjectKey` 归 `real.rs`。

## 文件级迁移表（old/ → 新树）

> `old/` 指不参与编译的旧代码参考根；路径前缀按实际放置位置调整。
> 动作分类：`直接搬` / `拆分` / `合并` / `内部符号挪动` / `删除`。
> 本表只说明“从哪搬到哪”，编译验证在整个搬移结束后统一做。

| 旧路径（old/ 内） | 新路径（目标树） | 动作 | 说明 |
|---|---|---|---|
| `mod.rs` | `overlayfs/mod.rs` | 直接搬 | 模块声明与 init，少量 use 调整 |
| `fs_type.rs` | `overlayfs/fs_type.rs` | 直接搬 | |
| `superblock.rs` | `overlayfs/fs/mod.rs` | 直接搬（改名） | `OverlayFs` + `FileSystem` impl |
| `mount/mod.rs` | `overlayfs/fs/mount/mod.rs` | 合并 | mount 模块入口与 `OverlayFs::new` 编排合并 |
| `mount/build.rs` | `overlayfs/fs/mount/mod.rs` | 合并 | `OverlayFs::new` 归入 mount 入口 |
| `mount/options.rs` | `overlayfs/fs/mount/options.rs` | 直接搬 | |
| `mount/layers.rs` | `overlayfs/layer.rs` + `overlayfs/real.rs` + `overlayfs/fs/mount/layer_parts.rs` | 拆分 | `Layer`/`LayerStack` 到 `layer.rs`；`RealPath` 到 `real.rs`；mount 期解析/`LayerParts`/`assemble` 到 `layer_parts.rs` |
| `mount/claims.rs` | `overlayfs/fs/mount/inuse.rs` | 直接搬（改名） | `UpperWorkdirClaim`→`UpperWorkdirInuse`，`InodeClaimGuard`→`InuseGuard` |
| `mount/policy.rs` | `overlayfs/fs/policy.rs` + `overlayfs/fs/mount/capabilities.rs` | 拆分 | `MountPolicy`/`UuidMode`/`XinoMode` 到 `policy.rs`；`UpperFilesystemCapabilities`/`DTypeProbeVisitor` 到 `capabilities.rs` |
| `projection/mod.rs` | `overlayfs/inode/lookup.rs` 等 | 拆分/内部符号挪动 | 模块 glue 消灭；`lookup_binding`/`project_inode` 等归 `inode/lookup.rs`，其余符号按各自目标文件走 |
| `projection/binding_cache.rs` | （删除） | 删除/内联 | `BindingCache`/`PositiveBinding`/`HiddenEvidence`/`BindingKey` 等消灭；必要符号内联到 `inode/mod.rs` 或 `inode/lookup.rs` |
| `projection/lookup.rs` | `overlayfs/real.rs` + `overlayfs/inode/lookup.rs` | 拆分 | `RealObject` 到 `real.rs`；lookup 流程/`is_whiteout_inode` 等到 `inode/lookup.rs` |
| `projection/identity.rs` | `overlayfs/inode/identity.rs` | 直接搬（改名） | `ObjectId`/`IdentityPolicy`/`LowerLayerIdentity` 等 |
| `projection/inode_cache.rs` | `overlayfs/inode/inode_cache.rs` + `overlayfs/real.rs` | 拆分 | `InodeCache` 到 `inode_cache.rs`；`RealObjectKey` 到 `real.rs` |
| `projection/lower_id.rs` | `overlayfs/inode/identity.rs` 或 `overlayfs/inode/xattr.rs` | 内部符号挪动 | `LowerIdOrigin` 归属按 symbol-map 不确定点处理 |
| `inode.rs` | `overlayfs/inode/mod.rs` + `overlayfs/inode/data.rs` | 拆分 | `OverlayInode` 主体/锁/事实替换到 `mod.rs`；数据/文件操作到 `data.rs` |
| `readdir_index.rs` | `overlayfs/inode/readdir.rs` | 直接搬（改名） | |
| `copyup/coordination.rs` | `overlayfs/inode/copyup/mod.rs` | 合并 | `CopyUpTransition`/`CopyUpPhase` 并入 |
| `copyup/mod.rs` | `overlayfs/inode/copyup/mod.rs` | 合并 | 仲裁入口并入 |
| `copyup/promote.rs` | `overlayfs/inode/copyup/mod.rs` | 合并 | promote 实现并入 |
| `copyup/trigger.rs` | `overlayfs/inode/copyup/mod.rs` | 合并 | `ensure_upper_authority` 并入 |
| `workdir.rs` | `overlayfs/inode/copyup/workdir.rs` | 直接搬（改名） | |
| `dir/mod.rs` | `overlayfs/inode/dir/mod.rs` | 直接搬 | |
| `dir/create.rs` | `overlayfs/inode/dir/create.rs` | 直接搬 | |
| `dir/link.rs` | `overlayfs/inode/dir/link.rs` | 直接搬 | |
| `dir/remove.rs` | `overlayfs/inode/dir/remove.rs` | 直接搬 | |
| `dir/rename.rs` | `overlayfs/inode/dir/rename.rs` | 直接搬 | |
| `dir/whiteout.rs` | `overlayfs/inode/dir/whiteout.rs` | 直接搬 | |
| `metadata_security/mod.rs` | （删除） | 删除/拆分 | 模块 glue 消灭；子模块按下三行归入 `inode/` |
| `metadata_security/permission.rs` | `overlayfs/inode/permission.rs` | 直接搬（改名） | |
| `metadata_security/metadata.rs` | `overlayfs/inode/metadata.rs` | 直接搬（改名） | |
| `metadata_security/xattr.rs` | `overlayfs/inode/xattr.rs` | 直接搬（改名） | |
| `legacy_fs.rs` | （不搬） | 保留 | 仅作 old/ 参考，不进新树 |

汇总：绝大多数条目是**直接搬**；少数是**拆分**（`mount/layers.rs`、`mount/policy.rs`、`projection/mod.rs`、`projection/lookup.rs`、`projection/inode_cache.rs`、`inode.rs`）；少数是**合并**（copyup 四个文件并入一个 `inode/copyup/mod.rs`、`mount/mod.rs`+`build.rs` 合并）；少数是**内部符号挪动/删除**（`projection/lower_id.rs`、`projection/binding_cache.rs`、`metadata_security/mod.rs`）。


## overlayfs/mod.rs

- `enum AccessType`
- `fn with_current_posix_thread`
- `fn lookup_child_path`
- `fn read_child_names`
- `fn mknod_object_type`
- `fn workdir_temp_name`
- `fn uuid_xattr_name`
- `fn init`

## overlayfs/fs_type.rs

- `struct OverlayFsType`
- `const OVERLAY_FS_NAME`

## fs/mod.rs

- `struct OverlayFs`
- `fn OverlayFs::root_visible_key`
- `fn OverlayFs::selected_real_fs`
- `FileSystem` impl 重要入口：
  - `fn sync`
  - `fn root_inode`
  - `fn sb`
  - `fn flags`
  - `fn set_fs_flags`

## fs/policy.rs

- `struct MountPolicy`
- `enum UuidMode`
- `enum XinoMode`
- `fn MountPolicy::assemble`

## fs/mount/mod.rs

- `fn OverlayFs::new`（当前 `mount/build.rs` 的构造编排）

## fs/mount/options.rs

- `struct MountOptions`
- `fn MountOptions::parse`

## fs/mount/layer_parts.rs

- `fn resolve_root_path`
- `fn verify_inode_instance_stability`
- `type LayerParts`
- `fn Layer::resolve_parts`
- `fn LayerStack::assemble`

> 注：`LayerStack` / `Layer` 的类型与纯方法见顶层 `layer.rs`；mount 期的路径解析、`LayerParts` 组装与 `LayerStack::assemble` 留在此处。

## fs/mount/inuse.rs

- `struct Uuid`
- `struct InodeClaimGuard`（目标改名 `InuseGuard`）
- `struct UpperWorkdirClaim`（目标改名 `UpperWorkdirInuse`）
- `fn Uuid::try_new`
- `fn Uuid::generate`
- `fn InodeClaimGuard::try_claim`
- `fn UpperWorkdirClaim::validate_pair`
- `fn UpperWorkdirClaim::determine_identity`
- `fn UpperWorkdirClaim::claim`
- `fn UpperWorkdirClaim::prepare_workdir`
- `fn UpperWorkdirClaim::persist_identity`

## fs/mount/capabilities.rs

- `struct UpperFilesystemCapabilities`
- `struct DTypeProbeVisitor`
- `fn UpperFilesystemCapabilities::probe`
- `fn UpperFilesystemCapabilities::validate_uuid_support`
- `fn UpperFilesystemCapabilities::probe_private_xattr`
- `fn UpperFilesystemCapabilities::probe_d_type`
- `fn UpperFilesystemCapabilities::probe_mknod_char`

## inode/mod.rs

- `struct OverlayInode`
- `enum PositiveKind`
- `fn OverlayInode::new_root`
- `fn OverlayInode::replace_facts`
- `fn OverlayInode::append_write`
- `fn OverlayInode::upper_parent_path`
- `fn OverlayInode::lock_copyup_transition`
- `fn OverlayInode::try_lock_copyup_transition`
- `fn OverlayInode::fs_arc`
- 数据/文件操作重要入口（当前 `inode.rs`，目标可能归 `inode/data.rs`）：
  - `fn read_at_impl`
  - `fn write_at_impl`
  - `fn open_impl`
  - `fn resize_impl`
  - `fn fallocate_impl`
  - `fn sync_all_impl`
  - `fn sync_data_impl`
  - `fn read_link_impl`
  - `fn lookup_impl`

> 注：inode 不再持有独立 facts 类型；由 `layer.rs::RealObjectStack` 承载真实对象栈。

## inode/lookup.rs

- `enum Lookup`
- `enum NegativeLookup`

> 注：不再有独立的 `LayerLookup` 中间类型；lookup 内部直接投影为 `Lookup`。
- `fn is_whiteout_inode`
- `fn RealObject::is_whiteout`
- `fn RealObject::is_opaque_directory`
- `fn OverlayFs::lookup_in_layers`
- `fn OverlayFs::project_inode`
- `fn OverlayFs::publish_lookup`（若仍需发布逻辑则保留；否则删除）

## inode/inode_cache.rs

- `struct InodeCache`
- `struct InodeCacheEntry`（私有）
- `fn InodeCache::new`
- `fn InodeCache::get`
- `fn InodeCache::get_or_create`
- `fn InodeCache::rekey_keep_old_alias`

## inode/identity.rs

- `struct ObjectId`
- `struct IdentityPolicy`
- `struct LowerLayerIdentity`
- `struct LowerIdOrigin`
- `fn IdentityPolicy::new`
- `fn IdentityPolicy::project_object_id`
- `fn IdentityPolicy::project_object_id_from_lower_id`
- `fn IdentityPolicy::origin_real_ino_resolves`
- `fn IdentityPolicy::resolve_layer_id_for_record`
- `fn collect_layer_devs(layer_stack: &LayerStack)`（从层栈收集 `LowerLayerIdentity`，供 `IdentityPolicy::new` 使用）
- `fn OverlayFs::store_lower_id`
- `fn OverlayFs::read_lower_id`

## inode/readdir.rs

- `struct ReaddirCookie`
- `enum ReaddirIndexValidity`
- `struct ReaddirIndex`
- `enum ReaddirIndexEntry`
- `fn OverlayInode::readdir_at_impl`
- `fn OverlayInode::invalidate_readdir_index`
- `fn OverlayInode::readdir_index_insert`
- `fn OverlayInode::readdir_index_remove`
- `fn OverlayInode::visible_child_count`
- `fn OverlayInode::ensure_readdir_index`
- `fn OverlayInode::readdir_sequence`
- `fn OverlayInode::resolve_parent_object_id`
- `fn OverlayInode::project_parent_from_lower_record`
- `fn OverlayInode::is_mount_root`
- `fn OverlayInode::parent_fallback`
- `fn ReaddirIndex::new`
- `fn ReaddirIndex::visible_inodes`
- `fn ReaddirIndex::rebuild`
- `fn ReaddirIndex::first_entry_after`
- `fn ReaddirIndex::remove_visible`
- `fn ReaddirIndex::insert_visible`
- `fn ReaddirIndex::compact_tombstones`

## inode/copyup/mod.rs

- `struct CopyUpTransition`
- `enum CopyUpPhase`
- `fn OverlayInode::ensure_upper_authority`
- `fn OverlayInode::ensure_upper_authority_inner`
- `fn OverlayInode::try_record_copyup_transition`
- `fn OverlayInode::promote`
- `fn OverlayInode::publish_via_rename`
- `fn OverlayInode::finish_promotion`
- `fn OverlayInode::promote_regular_file`
- `fn OverlayInode::promote_symlink`
- `fn OverlayInode::transfer_metadata`
- `fn OverlayInode::transfer_timestamps`
- `fn OverlayInode::copy_eligible_xattrs`
- `fn OverlayInode::publish_upper_authority`
- `fn OverlayInode::verify_upper_target`
- `fn OverlayInode::upper_real_object`
- `fn OverlayInode::mark_reconcile_pending`
- `fn OverlayInode::lower_source`

## inode/copyup/workdir.rs

- `enum WorkdirTempRequest`
- `struct WorkdirTemp`
- `fn WorkdirTemp::name`
- `fn WorkdirTemp::kind`
- `fn WorkdirTemp::inode`
- `fn WorkdirTemp::into_parts`
- `fn OverlayFs::create_workdir_temp`
- `fn OverlayFs::cleanup_workdir_temp`
- `fn OverlayFs::workdir_root_path`
- `fn OverlayInode::workdir_root_path`

## inode/dir/mod.rs

- `enum RemoveKind`
- `fn OverlayInode::create_impl`
- `fn OverlayInode::mknod_impl`
- `fn OverlayInode::write_link_impl`
- `fn OverlayInode::link_impl`
- `fn OverlayInode::unlink_impl`
- `fn OverlayInode::rmdir_impl`
- `fn OverlayInode::rename_impl`
- `fn OverlayInode::lock_dir_transaction`
- `fn OverlayInode::lock_parent_dir_transactions`

## inode/dir/create.rs

- `fn OverlayInode::create_object`
- `fn OverlayInode::create_upper_only`
- `fn OverlayInode::create_over_whiteout`

## inode/dir/link.rs

- `fn OverlayInode::link_source`
- `fn OverlayInode::link_over_whiteout`

## inode/dir/remove.rs

- `fn OverlayInode::remove_target`
- `fn OverlayInode::clear_empty_exchange`
- `fn translate_stale_upper_enoent`

## inode/dir/rename.rs

- `fn OverlayInode::rename_upper`
- `fn OverlayInode::cross_device_gate`

## inode/dir/whiteout.rs

- `struct WhiteoutCache`
- `struct WhiteoutHandle`
- `enum WhiteoutRepresentation`
- `fn WhiteoutCache::new`
- `fn WhiteoutCache::take`
- `fn WhiteoutCache::store`
- `fn WhiteoutCache::disable_sharing`
- `fn OverlayInode::whiteout_representation`
- `fn OverlayInode::create_whiteout_temp`
- `fn OverlayInode::publish_whiteout`
- `fn cleanup_upper_whiteouts`
- `fn is_whiteout_child`
- `fn validate_whiteout_children`
- `fn unlink_rechecked_whiteouts`

## inode/data.rs

- `fn OverlayInode::read_at_impl`
- `fn OverlayInode::write_at_impl`
- `fn OverlayInode::open_impl`
- `fn OverlayInode::resize_impl`
- `fn OverlayInode::fallocate_impl`
- `fn OverlayInode::sync_all_impl`
- `fn OverlayInode::sync_data_impl`
- `fn OverlayInode::read_link_impl`
- `fn OverlayInode::page_cache_impl`

## inode/permission.rs

- `fn OverlayInode::check_permission`
- `fn OverlayInode::check_local_permission`
- `fn OverlayInode::check_real_permission`

## inode/metadata.rs

- `fn OverlayInode::set_mode_impl`
- `fn OverlayInode::set_owner_impl`
- `fn OverlayInode::set_group_impl`
- `fn OverlayInode::set_atime_impl`
- `fn OverlayInode::set_mtime_impl`
- `fn OverlayInode::set_ctime_impl`
- `fn OverlayInode::best_effort_time_set`

> 注：`CallerOwnerFacts` 已消灭；`caller_owner_facts` 的 is_owner/has_cap 逻辑改为调用点局部计算。

## inode/xattr.rs

- `enum XattrClass`
- `enum XattrCopyPolicy`
- `enum MarkerReadSemantics`
- `fn origin_xattr_name`
- `fn impure_marker_name`
- `fn opaque_marker_name`
- `fn whiteout_marker_name`
- `fn XattrPolicy::classify`
- `fn XattrPolicy::is_private`
- `fn XattrPolicy::filter_private_names`
- `fn XattrPolicy::copy_eligible_xattrs`
- `fn XattrPolicy::has_marker`
- `fn XattrPolicy::has_impure_marker`
- `fn XattrPolicy::set_impure_marker`
- `fn XattrPolicy::clear_impure_marker`
- `fn XattrPolicy::set_opaque_marker`
- `fn OverlayInode::refresh_impure_marker`
- `fn OverlayInode::get_xattr_impl`
- `fn OverlayInode::set_xattr_impl`
- `fn OverlayInode::list_xattr_impl`
- `fn OverlayInode::remove_xattr_impl`

> 注：`XattrPolicy` 是 ZST，按 proposal 消灭为自由函数/关联函数，因此不列 struct。

## real.rs

- `struct RealObject`
- `struct RealPath`
- `struct RealObjectKey`
- `fn RealObject::identity_only`
- `fn RealObject::from_layer_path`
- `fn RealObject::child_hit`
- `fn RealObject::layer_index`
- `fn RealObject::real_inode`
- `fn RealObject::real_path`
- `fn RealObject::fsid`
- `fn RealObject::container_dev_id`
- `fn RealPath::from_path`
- `fn RealPath::upgrade`
- `fn RealPath::inode`
- `fn RealObjectKey::from_source`

> 注：`RealObjectKey` 只提供从 `RealObject` 构造的 `from_source`；从 `RealObjectStack` 构造的 key 由 `layer.rs::RealObjectStack` 提供，避免 `real.rs` 反向依赖 `layer.rs`。

## layer.rs

- `struct LayerStack`
- `struct Layer`
- `struct RealObjectStack`
- `fn LayerStack::validate_layer_overlap`
- `fn LayerStack::validate_workdir_against_lowers`
- `fn LayerStack::upper_layer`
- `fn LayerStack::lower_layers`
- `fn LayerStack::lower_layer_root_ino_for_origin`
- `fn Layer::child_real_object`
- `fn RealObjectStack::visible_source`
- `fn RealObjectStack::key`（经 `RealObjectKey::from_source(visible_source())` 构造）
- `fn RealObjectStack::is_merged`
- `fn RealObjectStack::contains_real_inode`
- `fn RealObjectStack::same_real_object_stack`
- `fn RealObjectStack::select_real_inode`

## 暂不列入（预计删除或已消灭）

- `BindingCache`
- `PositiveBinding`
- `HiddenEvidence`
- `BindingKey`
- `LookupOutcome`
- `WorkdirWorkspace`
- `PreparedTemp`
- `PromoteTarget`
- `CommitMarker`
- `XattrPolicy`（ZST）
- 所有 `*_impl` 转发壳与简单 trait 转发方法

## 待办 / 不确定点

- `RealObjectKey::from_facts` 的归属：`real.rs` 只提供 `from_source`；从 `RealObjectStack` 构造 key 放在 `layer.rs::RealObjectStack::key`，避免 `real.rs` 反向依赖 `layer.rs`。
- `fs/mount/layer_parts.rs` 与顶层 `layer.rs` 的边界：mount 期解析/校验 helper 与 `LayerStack`/`Layer` 的最终归属需依赖分析。
- `LowerIdOrigin` 最终放 `inode/identity.rs` 还是 `inode/xattr.rs`：proposal 阶段不强制决定。
- `inode/lookup.rs` 的 `publish_lookup` 是否保留取决于最终 lookup 流程。
