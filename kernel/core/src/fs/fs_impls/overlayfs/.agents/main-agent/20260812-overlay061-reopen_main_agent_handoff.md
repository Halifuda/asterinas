<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-12 overlay/061 调查与决策（暂不修复）

**Date / Time:** 2026-08-12 20:50 CST
**Status:** `RECORD — 061 调查与决策已定稿（user-directed 2026-08-12）：暂不修复，当前失败形态与 upstream Linux 一致、可接受；eager copy-up 是否正确仍是待讨论问题。本文件只记录调查结论与最终可能的修复方向，未启动任何修复。PR 流程（wave-pr）7-commit + regression commit 已全部 push（见 §1），上一份活跃 handoff（20260812-pr-draft-prep）已 CLOSED。`

## 1. Global State Pointer

- **上游基线:** `upstream/main` = `b96bbe3a1`（2026-08-12 仍为最新；kernel 树重构 `kernel/src` → `kernel/core/src` 已含）。
- **dev 分支:** `codex/overlayfs-refactor` @ `9d1b5eb43`（`fix CI bugs (rustdoc + regression test behavior)`；工作树干净）。Wave7/Wave8 已关闭；20 例可调度矩阵全量复测 PASS（user-confirmed 2026-08-11）。
- **PR 分支（~/asterinas-pr `codex/pr-overlayfs-refactor`）:** @ `4c713d165`，**7-commit 切分 + regression commit 已全部 push 到 `origin/codex/pr-overlayfs-refactor`**（`b96bbe3a1..4c713d165`：C1 mount → C2 projection① → C3 projection②/readdir → C4 copyup → C5 dir/whiteout → C6 metadata/security/xattr → C7 注册+删 legacy → 8th `align readdir_small_buffer regression test with Linux overlayfs`）。pr-draft-prep handoff 的 §8.7/§8.8 待办（push doc fixes、regression 验证/迁移）已由后续会话完成，不再挂起。
- **Blueprint Updates Made:** 否 —— `SYSTEM_BLUEPRINT.md` 无逐用例台账（061 行在 wave7 handoff §8 统一账，保留为历史记录）；061 决策以本 handoff 为正式记录。

## 2. Pass Slicing Decisions

- 无新 Creator/Checker pass，**不启动任何修复**。若未来重启 061，按协议走 `diagnosis → design → creator/checker`；候选组件归属：**读触发 page-cache/copy-up 语义 → `copyup_authority_file_views`；key/binding 一致性 → `visibility_projection_identity`**（仅记录，见 §4.5）。

## 3. Thread Activity Log（061 调查全貌）

### 3.1 Case 定义与重要性

- `overlay/061` = "memory mapped data inconsistencies"，`overlay/016` 的 mmap 变体：`mmap -r`（ro MAP_SHARED，copy-up 前创建）→ close → rw open（触发 copy-up）→ `mmap -w` → `mwrite 'a'×16` → `munmap`（xfs_io 映射表回落到 ro 映射）→ **in-place `mread`** → remount 后 `mread`。
- 断言两处：in-place 读与 mount cycle 后读都须为 `a`×16。场景 = **MAP_SHARED 在 copy-up 前后的读写一致性 + mmap 写持久化**（POSIX MMAP_SHARED 语义；对数据库/共享状态类应用重要）。

### 3.2 证据链（时间线）

1. **2026-08-09 首次 FAIL**（`components/wave7-xfstests-sequencing/overlay061_bug_trigger_flow_20260809.md`）：归因 overlay 投影/InodeCache 缺陷（key 跨 copy-up 不稳定 + 同 key 双载体 + 陈旧 lower 载体 → 双 copy-up → 两次 mmap 落不同 page cache）。
2. **2026-08-10 归因修正**（`overlay061_reinvestigation_20260810.md`，handoff §8.4）：3 轮全量打点**未复现**双 copy-up（单载体、单 promote、alias 15→17 OK）；真凶 = 通用 VM/page-cache 缺陷（MAP_SHARED 写缺页不 `CachePage::set_dirty()` → unmount 回写丢失）。当时用户决定不修、061 关闭。该文档并预测：VM 修好后 in-place 断言需用 guest xfs_io 语义重验，"it may still print old news … upstream Linux with xfs_io 6.13 shows the same mapping-revert behavior"。
3. **rebase upstream main 后 VM 缺陷已修**（代码证据）：`kernel/core/src/vm/page_cache/vmo/mod.rs` `VmoMapMode::SharedWrite` → `ensure()/try_ensure()` 现在 `locked_page.set_dirty()`（行 238/266）；`kernel/core/src/vm/vmar/vm_mapping.rs::prepare_page` shared write 用 `SharedWrite` 提交。
4. **2026-08-12 17:10 复测（此前未落档，仅原始日志）**：`components/wave7-xfstests-sequencing/run_evidence/overlay061/061_rerun_20260812/qemu.log`：
   - 失败形态已变：`+00000000: … This.is.old.news`（in-place 读旧值），**`After mount cycle: a×16` 已 PASS**（写持久化修好）。
   - 日志无 panic/EIO/崩溃迹象；当前唯一 mismatch = in-place 读旧值。

### 3.3 机制对照（代码 + 真机实验）

| 实现 | 读触发 page-cache 行为 | 061 in-place 读 |
|---|---|---|
| **legacy Asterinas** | `legacy_fs.rs:521-525` `page_cache()` **无条件** `build_upper_recursively_if_needed()`（注释 "Do copy-up for the potential memory mapping operations"）——不做读写区分 | **PASS**（baseline：`subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv` 062 行 = PASS） |
| **新实现** | `copyup/mod.rs:283-285` `page_cache_impl()` 仅 `select_real_inode().page_cache()`，"Never promotes … carries no write intent" | FAIL（ro 映射留 lower，in-place 读旧值） |
| **上游 Linux** | `fs/overlayfs/file.c::ovl_mmap` 只映射 `of->realfile`（ro open → lower，不 copy-up）；`ovl_open` 仅 O_WRONLY/O_RDWR 触发 copy-up | **同样 FAIL**（见 3.4 实验） |

### 3.4 本次 Linux 对照实验（2026-08-12，主代理，codex-asterinas-dev 容器）

- 真实 Linux overlay（tmpfs lower/upper/work）+ 与 xfs_io `io/mmap.c` 完全一致的映射表语义（`munmap_f` 后 current mapping 回落到前一个 ro 映射）复现 061 序列：`INPLACE_READ: b'This is old news'`，upper 内容 = `a`×16，lower = 旧值；remount 后读 = `a`×16。**与 Asterinas 08-12 复测输出逐字节一致**。
- 结论：当前 061 失败形态**不是相对 Linux 的偏差**，而是相对「legacy 已达成、Linux 尚未达成」的 MMAP_SHARED 一致性目标的差距。

### 3.5 为什么 Linux 没有合入 `copy_up_shared`（研究结论）

1. 2018 v2 系列（`overlayfs: stack file operations`，39 patches）曾含 `ovl: copy-up on MAP_SHARED`（`CONFIG_OVERLAY_FS_COPY_UP_SHARED`，default n）与配套新 VFS 钩子 `f_op->pre_mmap()`（mmap_sem 之前调用，避免 copy-up 的 VFS 锁嵌套进 mmap 锁）。Miklos 自述为 "A corner case of a corner case"，并注明 "This may result in unnecessary copy-ups … We can revisit this later if it turns out to be a performance problem in real life"。
2. 合入 4.19 前，VFS 维护者反对新 VFS 钩子（Christoph Hellwig：pull 含 "NAKed or at least non-acked VFS changes"；Al Viro："The worst of yours had been ->pre_mmap(), right? He *did* drop that..."）。最终 4.19 合入的 66-patch 系列保留了 `ovl_mmap` 与 ro/rw fd 一致性修复（overlay/016 过），**但删掉了 pre_mmap + copy_up_shared**；mmap 侧陈旧映射行为被保留并文档化为已知非标准行为。
3. xfstests 同步跟进：061 是 **upstream 唯一预期失败的 overlay 测试**（2018 split commit："one test to track the remaining non-standard behavior … w.r.t mmap"；2019 enhance commit 明说 "expected to fail on upstream kernel"，并改为检查 MMAP_SHARED 一致性 + close 后持久化）。
4. 后续 2020 年 RFC（Amir Goldstein `ovl: copy-up on MAP_SHARED`、Chengguang Xu `stacked mmap for shared map`）均未合入；当前内核（7.2.0-rc3）无 `copy_up_shared`，行为与 3.3 表一致。
5. **未合入根因汇总**：(a) 需要新 VFS API（pre_mmap）+ 锁序约束，VFS 维护者不买账；(b) 对每个 MAP_SHARED（含只读）eager copy-up 违背 lazy copy-up、浪费 IO/磁盘，收益仅覆盖「copy-up 前已建 ro 映射」这一罕见角落；(c) 已建立 VMA 无法事后重定向，唯一简单方案就是 eager copy-up，而无人愿意把它做成默认。

### 3.6 Asterinas 锁序调查（2026-08-12，主代理，读码核实）

**结论：Asterinas 不会遇到 Linux 同型的 mmap 锁序问题，但存在两个需要留意的设计点（本决策不修，仅记录）。**

- **mmap 解析期无 VM 锁**：`sys_mmap`（`kernel/core/src/syscall/mmap.rs`）构造 `VmarMapOptions`（`vmar.new_map()` 不持锁）；`options.is_shared(true)` → `options.mappable(file)` → `inode.page_cache()` 全部发生在 options 构建阶段（`kernel/core/src/vm/vmar/vmar_impls/map.rs:253`）。VMAR 写锁只在最后 `build()` 里 `parent.inner.write()`（`map.rs:283`）才拿，此时 fs 解析已完成。即 **fs 侧在 mmap 时看到 page-cache 请求的上下文无任何 VM 锁**（等价 Linux open() 上下文），因此在此处触发 copy-up 只拿 overlay 锁（`DIR → CUL → INODE → WL → UPPER` 拓扑），不存在 Linux 式 `mmap_lock` 嵌套问题。
- **缺页期禁止 copy-up**：Asterinas 缺页处理持 `vm_space.cursor_mut` 后再 `vmo.commit_on()`（`vm_mapping.rs:462-490`）；若在缺页期 promote，才是真正把 fs 锁嵌套进 VM 锁。设计约束 = **copy-up 只发生在 mmap 解析期（mappable/page_cache 时间点），绝不在缺页期**（恰好是 Linux `pre_mmap` 想达到的效果）。
- **设计点 1（潜在锁序再入风险，本次决策的直接触发点之一）**：`build()` 内有 debug_assert 会**再次调用** `path.inode().page_cache()`（`map.rs:294`，此时已持 VMAR 写锁）。若把 copy-up 做进 `page_cache()` 本身，debug 构建下会持 VMAR 锁二进 fs（当前无 overlay→VMAR 反向边，未构成死锁，但脏且脆弱）。→ 修复时 copy-up 应走 `mappable()`/专用入口，并把该 assert 改为对已解析 `vmo` 断言。
- **设计点 2（接口不可见 MAP_SHARED）**：`is_shared` 只存在于 `VmarMapOptions`，`FileLike::mappable()`/`Inode::page_cache()` 签名未接收共享/私有标志；"仅 MAP_SHARED copy-up"需扩接口（如新增 `prepare_for_mmap(is_shared, perms)`）；legacy 式无条件 copy-up 则无需改接口。

## 4. Explicit Agent-Level Decisions

1. **最终决策（user-directed 2026-08-12）：061 暂不修复。** 当前失败形态与 upstream Linux 一致、可接受；Asterinas 虽具备修复条件（见 §3.6），但 **eager copy-up 是否正确仍是待讨论问题**（性能/磁盘代价 vs MMAP_SHARED 强一致性）。本 handoff 只记录调查与最终可能的修复方向，**不启动任何修复**。此决策替代本日早些时候「重新打开/值得解决」的方向性讨论（讨论已进行，未进入任何诊断/实现调度）。
2. **未落档证据补录**：08-12 17:10 复测（`061_rerun_20260812/`）、Linux 对照实验、copy_up_shared 研究、Asterinas 锁序调查，均以本 handoff 为正式记录入口。
3. **接受当前失败 = Linux 一致行为**：in-place 读旧值不是「新引入的 bug」，而是与 upstream 一致的已知非标准行为；「解决 061」= 超越 Linux 默认语义，向 legacy 的强一致性对齐（仅当未来决定做时）。
4. **锁序结论记录**：Asterinas 无 Linux 同型锁序问题（mmap 解析期无 VM 锁）；两个设计点（debug_assert 再入、MAP_SHARED 不可见）与「缺页期不 copy-up」作为未来修复的前置约束记录在 §3.6，本次不修。
5. **最终可能的修复方向（仅记录，未实施）**：
   - (a) **eager copy-up（legacy 式）**：`page_cache_impl()`/mmap 解析期无条件触发 copy-up（复用 `ensure_upper_authority`/`promote`，保证单载体 + binding 替换）；无需接口扩展，但 MAP_PRIVATE 也会被 copy-up。
   - (b) **MAP_SHARED-only copy-up（等效 Linux copy_up_shared）**：仅共享映射触发；需接口扩展把 `is_shared`/perms 传给 fs（§3.6 设计点 2）。
   - (c) **映射重定向/失效**：copy-up 时重定向既有 lower 映射（Linux 未做，成本高，优先级低）。
   - 前置约束（无论哪个方向）：① 并发双载体/EIO 理论路径（`binding_inode_cache_consistency_static_review_20260810.md`：DIR 序列化或 displacement merge/恢复）必须先堵住；② copy-up 只在 mmap 解析期，绝不在缺页期；③ 处理 `build()` debug_assert 再入（§3.6 设计点 1）；④ 是否 eager 需先回答"正确性/收益"讨论（性能与磁盘代价 vs 一致性收益）。

## 5. Next Actions for the Next Thread (CRITICAL)

1. **无修复行动**：061 决策为暂不修复；不要 dispatch 任何 diagnosis/design/creator/checker 包，不要改动 overlayfs 代码。
2. **PR 流程**：7-commit + regression commit 已 push；如需继续（CI 绿、PR 描述/评论清理等），按 pr-draft-prep handoff §2-§8 口径推进。
3. **未来可选**：若用户决定重启 061 讨论，先回答 §4.5 前置约束 ④（eager copy-up 正确性/收益），再按协议走 diagnosis → design → creator/checker；届时按 §2 组件归属与 §3.6 约束执行。

## 6. Live File Discipline

- **This file is the live handoff for:** 061 决策记录 + 当前无活跃实现 wave（PR 已 push、061 暂不修复）的主代理 tenure。
- **Update rule:** 本文件原地更新直至下一 tenure；061 若重启，新调度/接受/拒绝/升级都记入本文件或由其指向的新 handoff。
- **Supersedes / Replaces:** `20260812-pr-draft-prep_main_agent_handoff.md`（已 CLOSED；其 PR 内容作为历史保留，PR 待办已并入 §1/§5）。
