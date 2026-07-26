<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlay FS 基础设计：Stage D 范围与实现时机草稿

**状态：** 核心范围已确认；hardlink 降级语义已补充，完整扩展排序仍可调整
**前置：** Stage A 与 Stage B/C 已完成
**定位：** 决定附加能力应该与 P0/P1 同批实现，还是在基础实现完成后再加入

## 1. Stage D 要回答的问题

Stage D 不再重新设计 ownership、lifecycle、锁序或 publication。Stage B/C 已经
给出了这些问题的统一答案。Stage D 只回答：

1. 哪些能力属于 basic Overlay FS 的必做范围；
2. 哪些能力会直接改变 P0/P1 的核心路径，应该在核心实现时一并处理；
3. 哪些能力可以在 P0/P1 完成后作为第一批扩展加入；
4. 哪些能力暂时延后，不影响 basic Overlay FS 的完成判定。

这里的“优先考虑”不等于立即开始实现。它首先表示要在当前设计中保留清晰的
交接边界和依赖闭包，避免未来扩展反过来重写已经稳定的核心 owner。

## 2. 不可改变的范围边界

### 2.1 P0/P1 是绝对必做范围

P0 的 18 个 Micro ID 和 P1 的 37 个 Micro ID，共 55 个 Micro ID，构成 basic
Overlay FS 的完成条件。它们覆盖：

- mount、layer stack、upper/workdir 和只读 mount；
- lookup、visibility、whiteout、opaque、stat 和 readdir；
- copy-up、file I/O、mmap、page-cache forwarding 和 fsync delegation；
- 两步 permission check 与 creator credential；
- create、unlink、rmdir、link、rename/`EXDEV`、symlink 和 xattr；
- workdir temporary、inuse exclusivity、whiteout cache 和目录 cache invalidation。

P0 单独完成时只有功能完整性，不代表安全完整性；`P1-18` 的两步权限检查必须
随 P1 完成，不能作为后续可选项。

### 2.2 P3 默认不进入 basic 实现

所有 P3 默认延后。当前明确排除：

- `P3-05` fs-verity；
- `P3-08` trap inode；
- NFS export 的只读子集；NFS export 必须作为一个完整能力处理；
- 任何没有明确纳入决策的其他 P3 能力。

`P3-09` workdir/index cleanup 只作为 NFS export 前的后续恢复与清理步骤，不
改变 basic 实现的完成判定。

## 3. 附加能力的实现时机

### 3.1 与 P0/P1 同批：xino 的核心部分

确认把 `P2-01 xino` 放入 P0/P1 的实现波次，原因是它直接影响：

- `stat` 的 `st_dev/st_ino` 映射；
- merged readdir 的 `d_ino`；
- B/C-2 的 identity projection；
- copy-up 后逻辑对象身份的连续性。

即使实现 xino，P0/P1 仍然必须定义 overflow 或不适用场景的明确 fallback；将
xino 与身份、stat、readdir 一起实现，也比核心完成后再改这些路径更容易收敛。

`P2-11` UUID modes 不因选择 xino 而自动全部纳入。Stage D 需要单独决定是否
实现完整的 `uuid=off/null/on/auto`；如果只需要 xino 的核心投影，应明确采用
已有 mount identity policy，而不是隐式扩大到所有 UUID mode。

### 3.2 P0/P1 完成后的第一批扩展

以下能力都比普通 P2 更值得优先考虑，但不建议直接混入 P0/P1 的基础完成条件。
它们会明显改变已有核心流程，应在 P0/P1 的 copy-up、identity、目录 mutation
和 page-cache 路径稳定后分别加入。

#### `redirect_dir`

对应 `P2-02`。它改变 lower/merged directory 跨目录 rename 的默认 `EXDEV`
路径，需要同时影响：

- directory promotion；
- redirect xattr 的写入和验证；
- rename publication；
- 后续 lookup 对 redirect 的解释。

它比普通 POSIX 完善项更接近核心 namespace mutation，因此和 metacopy 处于同一
优先级，必要时可以略早安排。没有显式启用时，基础行为仍然是 `EXDEV`，不能
让 redirect 的设计反向改变 basic rename 语义。

#### `metacopy` 与 data-only lower

对应 `P3-03` 和可选的 `P3-04`。metacopy 改变 copy-up 后的数据 authority：
upper 先拥有 metadata，实际 data 仍由 lower 提供，首次写入时再完成 data
copy-up。因此它必须依赖并扩展：

- B/C-4 的 copy-up authority transition；
- page-cache forwarding；
- write-open、mmap 和写入前 revalidation；
- `overlay.metacopy` 与 data-source metadata。

建议现在完成设计边界，等普通 full-data copy-up 和 page-cache 行为稳定后再实现。
`P3-04` 是否与 metacopy 同批，取决于是否确实需要 data-only layer；不能因为
考虑 metacopy 就自动扩大到所有 data-only 语义。fs-verity 保持排除。

#### `index` 与 origin verification

严格按 Micro inventory，index 是 `P3-01`；它的基础依赖闭包包括：

- 已属于 P1 的 `P1-07` origin FH encode/store；
- `P2-04` origin verification on lookup；
- B/C-2 的 object identity projection；
- B/C-4 的 copy-up 与 hardlink authority transition；
- 必要时的 workdir/index cleanup。

index 可以作为 P0/P1 完成后的高优先级扩展，但不应在基础 copy-up 尚未稳定时
伪装成一个局部功能。它的主要价值是保持 lower-upper 关联、hardlink 语义和
后续 export identity；没有 index 时的 nlink/hardlink 降级语义必须单独记录，
不能由 index 的存在自动改变 basic 范围。

### 3.2.1 Basic hardlink 的无 index 契约

`P1-28 link` 仍然属于 basic 范围，但它不等于 basic 阶段已经保证所有 lower
hardlink 关系在 copy-up 后永久保持。无 index 时应区分：

- **upper-authoritative 对象：** 对已经位于 upper 的对象执行 `link()`，新旧
  名称共享同一个 upper inode，硬链接关系可以正确保持；
- **lower 多链接对象：** 首个名称发生 copy-up 时，Overlay 可以建立一个 upper
  inode，但没有持久的 lower-object → upper-inode 关联来约束其他 lower alias；
  后续 alias 可能各自 copy-up 成不同的 upper inode；
- **结论：** 无 index 时不保证 lower 多链接关系跨 copy-up 的全局保持。该限制
  是明确的 basic 语义降级，不应被 xino 误认为已经解决；xino 只负责身份投影，
  不能替代 origin/index 关联。

`P2-07` nlink preservation 如果未来纳入，只能改善 `st_nlink` 的报告和 bookkeeping，
不能单独恢复已经断开的真实硬链接关系。完整的 lower-upper hardlink preservation
仍由后续 `P3-01 index` 扩展负责。

### 3.3 index cleanup 与 NFS export 的最后阶段

NFS export 对 identity 的要求高于普通 path-based Overlay 操作。建议保持以下
顺序：

```text
P0/P1 + xino
    -> redirect_dir / metacopy / index 等高优先级扩展
    -> workdir/index cleanup
    -> NFS export（完整能力）
```

其中：

- `P3-09` 先处理历史 workdir temporary 和 index residue 的恢复/清理；
- `P3-02` NFS export 依赖可验证的 index/origin identity；
- 不拆出“NFS 只读先做、NFS 写后做”的中间范围；
- 没有可靠 index 或 identity verification 时，NFS export 应拒绝启用，而不是
  降级为未经验证的 file handle 重建。

## 4. 暂时延后的功能

除上述 xino、redirect_dir、metacopy/data-only、index/origin 和 NFS 链条外，其余
P2/P3 默认不影响 basic 完成判定，包括：

- POSIX ACL、fileattr、无 index 的 nlink preservation；
- copy_file_range、fiemap 以及额外 file hooks；
- strict fsync、完整 UUID modes、userxattr、nested xattr escaping；
- layer casefold、FD layer specification、lowerdir colon escaping；
- volatile mount、override_creds、fs-verity、trap inode。

这些功能未来可以独立提出新的范围决策，但不能在 P0/P1 实现过程中隐式加入。

## 5. 当前建议的范围结论

Stage D 的推荐结论是：

1. **基础实现：** P0/P1 的 55 个 Micro ID；
2. **核心同批优先项：** 确认纳入 `P2-01 xino`，但 UUID modes 是否完整纳入另行确认；
3. **基础完成后的高优先级扩展：** `P2-02 redirect_dir`、`P3-03 metacopy`、
   `P3-04 data-only lower`（若需求需要）以及 `P3-01 index`；
4. **最后阶段：** `P3-09 workdir/index cleanup`，随后才考虑完整 `P3-02`
   NFS export；
5. **其余功能：** 延后，不进入当前 basic 设计的实现承诺。

这个结论表达的是实现时机和设计优先级，不是 Creator pass slicing。实现开始前
仍需完成 Designer wording repair、VFS private-state/publication interface 的
Stage F 定义，以及最终的综合设计审查和 traceability appendix。

## 6. Stage D 完成条件

- P0/P1 的 55 个 Micro ID 被标记为必做；
- xino、redirect_dir、metacopy、index、NFS export 的实现时机和依赖闭包已记录；
- 无 index 时 lower 多链接关系跨 copy-up 不保证保持，但 upper-authoritative
  `link()` 仍必须保持真实 upper inode 共享；
- P3 默认排除，且 traps/fs-verity/NFS read-only split 没有重新进入范围；
- basic rename、hardlink、fsync、identity 和 copy-up 的默认语义没有被附加功能
  偷换；
- 没有生产 Rust 修改、Creator/Checker packet、pass slicing 或 runtime validation
  被 Stage D 草稿隐式授权。
