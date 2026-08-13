<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-13 wave-9 注释审计 — 原则记录（只读轮）

**Date / Time:** 2026-08-13 CST
**Status:** `ACTIVE — wave-9 注释审计（2026-08-13→15）：21 条原则（含 2026-08-14 新增 C1–C6）定稿；经 R1–R6 六轮 Reviewer 复核 + 六轮 Creator 清理，注释已高度收敛（R6 仅 9 条 LOW 且全部修复；诚实轮确认无回潮）。AUDIT-* 0 残留；USER_TODO/USE_TODO 57 行保留待另案；**2026-08-15：mount 树 USER_TODO 另案已处理（40 条记录+删除，见 §4.17）+ mount 注释按 21 条原则再审查完成（3 条 N2 缺口待裁决，见 §4.17）+ A/B 灵活性原则全量审查已完成并验收（13 findings：HIGH 1/MED 2/LOW 10；10 REMAINING=PR3708 A/B 档、3 NEW；0 REGRESSION）+ 13/13 已按 user 批准执行（机械 3 + 灵活 10，仅注释行，见 §4.24）**；编译验证推迟至代码清理彻底完成后；user 将全局审阅所有代码，注释阶段非定稿。**2026-08-17：day-2 Flash 注释复审 54 findings 恢复清理完成（R1 34 + R2 20 全闭，仅注释行，已 amend，见 §4.35）**。`

## 1. Global State Pointer

- **dev 分支:** `codex/overlayfs-refactor` @ `wave-9: comments audit (WIP)`（2026-08-13 起多次 amend 的最新提交；最新 amend = 2026-08-17 day-2 Flash 清理 R1+R2 并入，见 §4.35；amend 后工作树仅剩 `.vscode/settings.json` 与未纳入本 amend 的 code-structure handoff）。上一活跃 handoff：`20260813-rebase-upstream_main_agent_handoff.md`（rebase/API-repair/验证 tenure，已 CLOSED，见 §6）。
- **审查对象/提交:** 起点 commit `5723f88e1`（10 文件 +271/−399），经注释审计与多轮清理 amend 后最新累计 **37 文件 +2344/−4779**（注释净减；含 `20260812-overlay061-reopen` 与 wave-9 原则 handoff 等管理记录）。
- **轮次总览:** R1 审计 304 → 三分类 104/82/113/5 → R2 276 → 最终处置 76/101/98/1 → C2 执行 → R3 291（C1–C6）→ C3 101/180/9/1 → R4 52 → C4 13/39 → R5 24（诚实轮，4 REGRESSION）→ C5 2/21/1 → R6 9 → C6 9 全闭。`AUDIT-*` 0 残留；`USER_TODO`/`USE_TODO` 57 行保留。
- **可见性上游依据（已核实）:** `b0fd25f57` Add short-vis-path crate；`59ef87536` Replace multi-level submodules with short-vis-path in kernel（ext2/virtiofs/netlink/vsock/vmar/vt 已迁移，overlayfs 为本分支代码故需自行对齐）；`00e0bad99` Document short-vis-path coding guidelines（`book/src/to-contribute/coding-guidelines/for-maintainability/rust-specific/crates-and-modules.md` §short-vis-path，设计 #3188）。
- **Blueprint Updates Made:** 是 —— `SYSTEM_BLUEPRINT.md` / `PASS_SLICING.md` 已随 2026-08-16
  R1+R2 清理轮更新（见两文件内 `wave9_principles_cleanup_20260816` 条目）；本 tenure 以本
  handoff 为正式记录。

## 2. Pass Slicing Decisions

- 无新 Creator/Checker/Reviewer pass，不启动任何修复或调度。

## 3. Thread Activity Log（wave-9 审计全貌）

- **只读审查 commit `5723f88e1`:** 逐文件阅读完整 diff（mod.rs + mount 7 文件 + 2 handoff）；枚举评论标记 **52 × `USER_TODO` + 1 × `USE_TODO`（拼写错误，build.rs:191）**，全部位于 overlayfs 顶层 mod.rs 与 mount 树。
- **可见性迁移盘点:** 41 处 `pub(in overlayfs)`（mod.rs 1 + mount 40）；`#![short_vis_path::add(overlayfs)]` 仅加在 `overlayfs/mod.rs`；**mount/*.rs 各文件均缺该属性**（上游先例 ext2/procfs 为逐文件携带）；6 处旧式 `pub(in crate::fs::fs_impls::overlayfs)` 漏网（claims.rs:365；policy.rs:147/153/173/289/300）；全树其余 158 处旧式未迁移。
- **user 澄清（2026-08-13）：** ① 可见性是上游明确提出的规范要求（非个人倾向）；② 原 P9（workdir 清理）不属于原则，只是对应代码位置的疑虑；③ accessor 疑虑的本意是「能否直接访问 struct 字段——一个简单 accessor 不提供任何封装性或其它好处」。
- **并入 PR Reviewer 文档反馈（2026-08-13，user 指示）：** 依据本地 book 两份指南（`for-maintainability/comments.md`、`rust-specific/comments.md`）与仓库内 References 写法（vt/manager、load_elf、ext2/mod.rs），新增 N1–N11（见 §4.1.1），并把原则重排为「注释/文档在前、代码在后」（代码原则归入 §4.1.2）。
- **全树注释审计（2026-08-13，user 指示）：** 6 个 V1 直派 Reviewer 子代理按 21 条原则审计全树注释，**304 条 finding**（mount 51 / projection 60 / dir 87 / copyup 32 / security 41 / top_readdir 33），报告在 `components/wave9-comments-audit-20260813/`；USER_TODO/USE_TODO 已按要求全部排除未汇报。
- **审计结果三分类（2026-08-13，user 指示）：** 1 个无 role 子代理（`task_classify_comments_audit_20260813`）把 304 条 finding 分为 全删 / 大幅精简 / 重写 三档（另有 5 条 ADJUDICATE），产物 `components/wave9-comments-audit-20260813/comments_audit_classification_20260813.md`；main agent 仅结构验收（汇总闭合 304）未通读。
- **注释清理执行（2026-08-13，user 指示）：** 6 个 Creator 子代理（mount/projection/dir/copyup/security/top_readdir）按三分类执行——delete 104 / simplify 82 / rewrite 113 仅加标注 / ADJUDICATE 5 仅加标注（计数与分类表完全闭合）；packet 全 pointer 式未复制条目；USER_TODO/USE_TODO 53 行原样保留（mount 内代码相关 USER_TODO 不动，user 重申）；临时标注 `AUDIT-REWRITE`（130 处，含多位置展开）与 `AUDIT-ADJUDICATE`（5 处）供 user 人工逐条审阅。收据 6 份在 `components/wave9-comments-audit-20260813/*_cleanup_receipt.md`。
- **全树注释二次复核（2026-08-14，user 指示）：** user 复核 readdir_index.rs 后认为清理不够（self-explain 未贯彻），派 6 个 Reviewer 子代理对**当前工作树**全部注释重做判断，self-explain（N1/P2）为第一判据；**276 条 finding**（mount 56 / projection 65 / dir 74 / copyup 28 / security 25 / top_readdir 28），处置 delete 71 / simplify 92 / rewrite 113；标记类型 REMAINING 59 / NEW 99 / ANNOTATED 118；HIGH 2（mount 1、dir 1）。USER_TODO/USE_TODO 与 AUDIT-* 标记行全部排除未汇报。报告在 `components/wave9-comments-reaudit-20260814/`。
- **最终处置（2026-08-14，user 指示）：** user 要求已标记 AUDIT-REWRITE 也纳入考量（该删则删、该精简则精简），派 6 个 Reviewer 子代理对全部 276 条 finding（含 ANNOTATED）逐条对照工作树原文给出最终处置：**delete 76 / simplify 101 / rewrite 98 / adjudicate 1**。ANNOTATED 降档：→delete 4（mount R34、dir W1/W5/W17）、→simplify 9（mount R11/R50、copyup T-A2/P-A7、top_readdir #19、security #15/#21、dir L1/R14）；ADJUDICATE 5 条中 4 条已定（top_readdir 2 delete、dir W9 delete + L1/R14 simplify），mount F27/F28 为无对应 finding 的标记，留 1 条 ADJUDICATE。AUDIT-* 普查：各 scope 标记全部覆盖（mount 25/27 覆盖 + F27/F28 单列；dir 33/33；其余全覆盖）。报告在 `components/wave9-comments-dispositions-20260814/`。
- **清理执行 Round 2（2026-08-14，user 指示）：** 6 个 Creator 子代理按最终处置清单执行——**delete 75 / simplify 101 / rewrite 98 / skipped 2**（mount R26 ADJUDICATE 保留；dir C6 目标注释已不存在），`AUDIT-*` 标记全树 **0 残留**；USER_TODO/USE_TODO 57 行未动；simplify 按“削到一句话”执行，rewrite 按处置方向落地（模块 doc 重组、N8 elixir References 链接、术语改名）。**唯一代码级改动**：dir W9 按处置删除 `whiteout_cache` accessor 并改 3 个调用点为直接字段访问（需编译验证）。收据 6 份在 `components/wave9-comments-cleanup2-20260814/`。rewrite 合并清单 `components/wave9-comments-dispositions-20260814/REWRITE_consolidated_20260814.md`（98 条）。
- **Reviewer Round 3（2026-08-14，user 指示）：** user 复核 entry.rs 后给出 6 条强化判据（C1–C6，见 §4.1.3）并重开 Reviewer 轮；6 个子代理对清理后工作树全量复核：**291 条 finding**（mount 59 / projection 83 / dir 40 / copyup 46 / security 40 / top_readdir 23），处置 delete 99 / simplify 101 / rewrite 91；HIGH 11（projection 10、mount 1）。user 点名的 entry.rs 六项全部处置（WHITEOUT_XATTR_FULL_NAME/is_whiteout_inode/with_path/L353 反向解释 → delete；RealObject/lookup_in_layers → rewrite）。报告在 `components/wave9-comments-reaudit3-20260814/`。
- **清理执行 Round 3（2026-08-14，user 覆写方向）：** user 研习 R3 rewrite 清单后裁定「绝大多数实为 simplify/delete；仅 Trait carrier/Trait 实现 doc 需与其它 FS 实现一致」。6 个 Creator 子代理执行：**delete 101 / simplify 180 / rewrite 9 / trait_doc 1**（91 条原 rewrite → 81 条降 simplify/delete 等，仅 9 条模块级 doc 真正 rewrite 且 ≤8 行，1 条 trait-impl doc 保留一句行为描述）。AUDIT-* 0 残留；USER_TODO/USE_TODO 57 行未动；仅注释变更。收据在 `components/wave9-comments-cleanup3-20260814/`。
- **Reviewer Round 4（2026-08-14，user 指示）：** 不编译、注释阶段继续；派 6 个 Reviewer 子代理遵循全部注释原则（21 条 + C1–C6 + Trait-impl 一致 + PR Reviewer 文档原则）对三轮清理后工作树全量复查：**52 条 finding（delete 12 / simplify 40 / rewrite 0，无 HIGH）**——mount 2 / projection 19 / dir 14 / copyup 9 / security 3 / top_readdir 5。entry.rs/lower_id.rs R3 整改全部达标；copyup promote.rs:186-187 C5 指针为 R3 执行缺口（收据声称已删但仍在树中）；dir References 去重 6 组定案；projection 剩 7 条 C1 锁散点 + 3 条 9 行 C6 边界块 + 9 条 P2 残余。报告在 `components/wave9-comments-reaudit4-20260814/`。
- **清理执行 Round 4（2026-08-14，user 指示）：** 按 R4 清单执行收尾清理（**delete 13 / simplify 39，合计 52 全闭**；R4 Summary 行的 12/40 为内部笔误，以 per-finding 表 13/39 为准）。copyup promote.rs:186-187 C5 执行缺口已补齐；References 行号区间补全（ovl_set_attr 更正为 copy_up.c）；copyup 首派代理挂起（约 25 分钟无产出），按协议关闭后重派完成。AUDIT-* 0 残留；USER_TODO/USE_TODO 57 行未动；仅注释变更。收据在 `components/wave9-comments-cleanup4-20260814/`。注释阶段暂告一段落——user 明确这不是定稿，后续将再审阅所有代码。
- **Reviewer Round 5（诚实轮，2026-08-14，user 指示）：** user 要求再派 Reviewer 确保收敛但**不以求收敛为目的**；6 个 Reviewer 子代理独立复核四轮清理后工作树：**24 条 finding（delete 2 / simplify 21 / rewrite 1）**——mount 1 / projection 7 / dir 6 / copyup 2 / security 8 / top_readdir 0（CLEAN）。**诚实轮抓到 R4 漏检的回潮**：projection identity.rs:78 C5 指针（R4 声称 C5 清零不实）、security permission.rs Pipeline/DAC 与 xattr.rs is_private 模块 doc↔方法 doc 近逐字重复（R3 收据声称闭合、R4 漏检）；security Ownership gate 段仍 10 行（>8）；另有 security 4 条模块 doc 概述段（9-11 行）C6 阈值边界交主代理裁决。报告在 `components/wave9-comments-reaudit5-20260814/`。注：期间沙箱运行器二进制缺失（codex-linux-sandbox ENOENT），子代理经批准以非沙箱完成只读审计与报告写入。
- **修复轮（Cleanup Round 5，2026-08-14，user 指示）：** 按 R5 清单执行修复（**delete 2 / simplify 21 / rewrite 1，合计 24 全闭**）；**4 条 REGRESSION 全部真删并 grep 确认 0 命中**（projection identity.rs:78 C5 指针；security permission.rs Pipeline 段整删、DAC 段与 xattr.rs is_private 重复句）；Ownership gate 与 4 条 C6 边界模块 doc 概述段均压至 ≤8 行且 N2 rationale 全保留；top_readdir 复核仍 CLEAN。AUDIT-* 0 残留；USER_TODO/USE_TODO 57 行未动；仅注释变更（15 文件 57+/86−）。收据在 `components/wave9-comments-cleanup5-20260814/`。
- **Reviewer Round 6（2026-08-14，user 指示）：** 再开一轮诚实 Reviewer 复核修复轮结果：**9 条 finding（0 delete / 9 simplify / 0 rewrite，全 LOW，0 REGRESSION）**——mount 1 / projection 2 / dir 2 / copyup 1 / security 2 / top_readdir 1。修复轮全部闭合声明**独立 grep 复核属实**（identity.rs:78 C5、security Pipeline/DAC/is_private、Carries only the phase.、promote.rs:186-187 等均 0 命中）；本轮新发现均为历轮漏网 LOW（含 security xattr.rs 两个 N2 块超 C6 阈值的诚实修正、copyup promote.rs:179-181 交主代理裁决）。报告在 `components/wave9-comments-reaudit6-20260814/`。
- **清理执行 Round 6（2026-08-14，user 指示）：** 按 R6 清单执行收尾清理（**simplified 9 / skipped 0，全部闭合**）：mount 1、projection 2、dir 2、copyup 1、security 2、top_readdir 1；三处裁决项按既定处置落地（copyup promote.rs:179-181 删复述仅留标签；security 两个 N2 块压至 ≤8 行且 Known divergence/Immutable-lower/race 全保留；top_readdir remove_visible C4 从句删、O(n) 并入 summary 句、N2 句内保留）。AUDIT-* 0；USER_TODO/USE_TODO 未动；仅注释变更（8 个 .rs + handoff）。收据在 `components/wave9-comments-cleanup6-20260814/`。
- **未执行:** 无构建/编译验证（只读轮）；未改动任何代码；未提交。

## 4. Explicit Agent-Level Decisions

### 4.1 wave-9 原则（user-confirmed 定稿；2026-08-13 并入 N1–N11，注释/文档原则在前，代码原则在后；**2026-08-15 并入 PR #3708 反馈：N12 新增，P3/N4/N5/N9/P5/P10 补充**）

#### 4.1.1 注释/文档原则（comment / documentation）

1. **P2 注释信息密度**：注释只补代码之外的信息；删除复述式文档（closed set / single representation / immutable after construction / no unwrap / no lock domain / 可见性用途说明等）。
2. **P3 注释自包含**：不引用其他注释或**其它文件路径/行号**作为依据（如 “the 11 ordered steps below”、`syscall/symlink.rs:44` 这类跨文件 file:line 引用会随对方改动而失效，一律禁止；外部依据只走模块级 References 链接）；多步骤各自成段；步骤编号/标记须自证顺序（`Step 4b (policy draft)` 存疑）。
3. **N1 explain-why（book: explain-why）**：注释解释意图与理由，不复述行为；若注释是为了解释“代码在做什么”，先重写代码使其自明，而不是用注释补偿坏代码。
4. **N2 design-decisions（book: design-decisions）**：对非显然的选择（数据结构、锁策略、与 Linux 行为的偏差）记录 rationale 与备选方案；设计决策注释（director's commentary）最有价值，删除/简化时须保留——与 wave-9 已删除注释需逐条对照复核（哪些误删了设计决策）。
5. **N3 cite-sources（book: cite-sources）**：行为由外部规范或非平凡算法定义时引用来源（POSIX 章节、Linux man page、硬件手册、学术论文）。
6. **N4 rfc1574-summary（book: rfc1574-summary）**：doc 首行一句话且必须为**高层 summary**（先总述、后展开）；函数/方法用第三人称单数现在时动词（`Returns`/`Creates`/`Acquires`）；类型/模块/字段用名词短语；**禁止 AI 推理过程倾泻式 doc**（复述推理过程、无 summary、无结构的 “LLM mumbling” 文字一律重写）。
7. **N5 no-impl-in-docs（book: no-impl-in-docs）**：doc 注释写 API **做什么 + 怎么用**，不写内部**怎么实现**；实现机制类内容移到实现注释或删除（与 P2 不同：P2 是删复述，N5 是文档内容边界）；**复杂/用户面向函数（如 `OverlayMountOptions::parse`）的 doc 应达 spec 精度**——把引号/转义/重复 key/空值等行为边界写明，使 doc 可作为 spec 与测试依据；行为未写明 = doc 缺口。
8. **N6 backtick-identifiers（book: backtick-identifiers）**：类型/方法/标识符用反引号；类型优先 rustdoc 链接 `[TypeName]`。
9. **N7 comment-punctuation（book: comment-punctuation）**：完整句子以标点结尾，避免碎片化 prose。
10. **N8 References 链接替代内联 Linux 细节（Reviewer 明确要求）**：正文不直接复述 Linux 实现细节（如 “mirrors Linux `ovl_fs_type`”）；表达兼容/对齐时正文只写行为语义，Linux 源码用 References 链接给出（elixir.bootlin.com + 文件 + 行号区间）。模式参考：`kernel/core/src/device/tty/vt/manager/mod.rs`（方法级 `References:`）、`kernel/core/src/process/program_loader/elf/load_elf.rs`（常量级）、`kernel/core/src/fs/fs_impls/ext2/mod.rs`（模块级 `# References`）。与 N2 的关系：与 Linux 的**偏差**是设计决策，正文讲 rationale；与 Linux 的**对齐**是行为事实，用链接证明。全树现存内联 `ovl_*` 引用（mount 外含 metadata_security/xattr.rs、readdir_index.rs、copyup/mod.rs（内联 `file.c:128-171`）、dir/remove.rs 等）均为 N8 清理对象。
11. **N9 模块文档结构化（Reviewer：reorganize）**：模块级文档按「概述 → 关键概念/不变量 → 模块结构表 → References」组织；长叙述段落拆成小节，让读者先抓语义再按需深入；**术语先定义后使用**——开篇定义该模块的核心术语（如 projection 模块先定义 “projection”），“binding/admission/project/gate/fact” 等自造词汇须先定义再出现；模块 doc 应高层描述模块语义，**不得只是枚举目录内容**。
12. **N10 三档处理策略（Reviewer）**：每条现有注释先归类再处理，避免一刀切——① **reorganize**（语义正确但组织混乱，重组为直观清晰的结构）；② **simplify**（删过多实现细节）；③ **remove**（无洞察则整段删除）。
13. **N11 基准选择（Reviewer 指引）**：不确定“好文档长什么样”时，参考仓库内文档立刻可读、写得清楚的模块（如 vt/manager、elf loader）；ext2 为 LLM 生成未精修，不作最佳基准。
14. **N12 TODO 注释格式（PR #3708 cchanging）**：方法 doc 用 `///`；TODO 用独立的 `// TODO: ...` 行放在 doc 之后（不进 doc、不混入 doc 正文）；TODO 为带退出条件的 forward-work 标记（无退出条件的存疑项按 P10 处置）。

#### 4.1.2 代码原则（code，置于最后）

15. **P1 可见性对齐上游 short-vis-path（上游强制）**：深层子系统（`crate::fs::fs_impls::overlayfs`）必须用 `#![short_vis_path::add(overlayfs)]` + `pub(in overlayfs)` 替代 `pub(in crate::fs::fs_impls::overlayfs)`。上游指南三条件全部满足：深度 > 2；`pub(super)`/`pub(self)` 不适用；受限可见路径使用 ≥ 2 次（全树 199 处）。
16. **P4 简单即内联**：单调用、参数自包含、无复用的 helper 优先内联（`determine_identity`、`persist_identity`、`selected_real_fs` 等）。
17. **P5 职责归位**：常量/逻辑归属持有状态的类型或模块（`XINO_SHIFT` → Identity mod；`collect_layer_devs` → `LayerStack` 且避免无类型 tuple；capability gates → capability mod）；同一作用域内同名概念不重复命名（`identity` 变量名用了两次）；**注释同样附着语义承载处**——删冗余包装抽象时其注释移到实际逻辑处（如 `MountOptionKey` variant 注释移到对应 `match` 分支），不贴在冗余抽象上。
18. **P6 单一校验边界、构造期前置**：同一不变量只在一个边界强制（parse / checked constructor / build 三选一，避免 “same or equivalent checks also in build”）；校验应在相关状态产生之前完成（“check this after assemble and many other works done?”）；重复谓词提炼共享 helper。
19. **P7 错误消息可诊断**：错误要回显具体值（malformed uuid、非法 option 值）；errno 对照 Linux（`ENOSPC` 存疑）；日志文案可读（EIO 消息）。
20. **P8 资源管理靠 RAII**：显式 `drop` 回滚脆弱；能由结构/Drop 顺序保证的释放顺序交给 RAII（claim 回滚）。
21. **P9′ accessor 直接访问字段优先（user 澄清版）**：若 struct 字段本身就在可见域内（`pub(in overlayfs)`/`pub(super)`），一行式 `fn x(&self) -> &T { &self.x }` 不提供封装、校验或抽象收益，应直接访问字段；仅当 accessor 承载额外语义（转换、校验、隐藏私有表示）或字段需保持私有时才保留。（对应 layers.rs `inode()`、policy.rs 多个 accessor、superblock.rs `layer_stack()` 等 TODO。）
22. **P10 术语准确、无死代码**：`snapshot`/`capability gates`/`MOUNT`/`source` 等要么改名、要么给出存在理由、要么删除；对“不知道为什么存在”的 probe/accessor/状态机要追到消费者（`what are these used for`）；**注释术语须与代码真实语义一致且全树统一**（per-mount 而非 per-filesystem——filesystem 实例可被多个 Mount 共享），语义变化时同步更新所有注释。

### 4.1.3 user 强化判据（2026-08-14，C1–C6，并入本轮 Reviewer 判定）

1. **C1 锁注释几乎全删**：overlay 锁拓扑简略，唯二需要解释的是 `DIR` 锁与 `CUL` 锁（且只在非显然处）；锁域/锁契约/锁顺序/held-across 等重复锁注释一律删。
2. **C2 自解释即删**：常量名+常量值联合自明的（如 `WHITEOUT_XATTR_FULL_NAME`）、内部 helper 代码可懂而注释复述算法的（如 `is_whiteout_inode()`）、类型签名自明的（`Weak`/`Option`/`Mutex`/`enum`）→ 删。
3. **C3 删“解释型”注释**：注释解释代码显而易见结构（如 `with_path()` 解释 “its anchor mount is held weakly” 而字段已是 `Weak<Mount>`）→ 删。
4. **C4 删反向解释**：解释未被选择的算法/替代方案（如 `offset + 1` 的 dense 论述）→ 默认删；仅当对照是必要 rationale（N2）时保留并说明。
5. **C5 删括号式 “see …” 指针**：`(Linux … see module references)`、`(see …)` 等括号内要求读者另看的表述全部删除（Linux 与非 Linux 一律）；References 只在模块级 `## References` 区。
6. **C6 超长段落必须处理**：>8 行的注释块/方法 doc（如 `lookup_in_layers()`、`RealObject`、`alias_key`、`replace_facts`）逐条列出并 rewrite（拆小节或大幅压缩），不许默默保留。

### 4.2 非原则的代码位置疑虑（不上升为原则，仅待裁决）

- **workdir 清理策略（原 P9，user 降级）**：`claims.rs` `remove_work_entries` 递归删除 + `ENOTEMPTY` vs 直接整体删除 `<workdir>/work/*` 的疑虑；裁决时需对照 Linux 行为（`ovl_workdir_cleanup` 三层契约）并记录结论。
- 其余逐条 USER_TODO 均为候选裁决项，见 commit `5723f88e1` 原文；本 handoff 不再复制全文（指针化）。

### 4.3 风险与待办结论（只读轮，未验证）

- **最可能编译阻塞点**：mount/*.rs 使用 `pub(in overlayfs)` 但缺 `#![short_vis_path::add(overlayfs)]` 属性；上游先例为逐文件携带。结论：**迁移必须完成而非回退**（上游强制），但需先编译验证。
- 完成迁移的机械范围：① 每个使用 `pub(in overlayfs)` 的文件补属性；② 清 6 处漏网旧式；③ 全树其余 158 处旧式一并迁移（否则风格分裂、CI/上游审查仍会打回）。
- `mod.rs:3` 的 USER_TODO（“we've got tip …”）是过程性笔记，迁移完成后应删除。

### 4.4 审计结果三分类（2026-08-13，已验收）

- **delete = 104 / simplify = 82 / rewrite = 113 / adjudicate = 5**（合计 304，与审计 finding 总数闭合）。
- 按 scope：mount 18/6/27，projection 23/10/27，dir 27/36/21（+3 ADJ），copyup 9/12/11，security 12/11/18，top_readdir 15/7/9（+2 ADJ）。
- **5 条 ADJUDICATE（依赖代码级裁决，改代码则注释随删）**：
  1. P8 RAII（dir #37 link.rs:20-29，显式 cleanup 辩护）；
  2. P4/P9′ helper/accessor 内联（dir #56 remove.rs:406-432、dir #79 whiteout.rs:232-238、top_readdir #21 readdir_index.rs:374-377）；
  3. P6 单一校验边界（top_readdir #11 readdir_index.rs:150-151，guard 辩护理由循环）。
- 详细分类表见 `components/wave9-comments-audit-20260813/comments_audit_classification_20260813.md`。

### 4.5 注释清理执行验收（2026-08-13，轻量验收）

- 6 份收据在册；每份动作统计与分类表对应 scope 完全一致：mount 18/6/27/0、projection 23/10/27/0、dir 27/36/21/3、copyup 9/12/11/0、security 12/11/18/0、top_readdir 15/7/9/2（delete 104 / simplify 82 / rewrite 113 / adjudicate 5 闭合）。
- 全树 diff 31 文件 497+/1176−（纯注释行）；`AUDIT-REWRITE` 130 处、`AUDIT-ADJUDICATE` 5 处；`USER_TODO`/`USE_TODO` 53 行未动。
- 子代理报告的残留风险（重叠 finding、标注位于 doc 块内等）已记录于各收据，供人工审阅时注意。
- 未编译、未提交；改动停留在工作树（wave-9 WIP 延续）。

### 4.6 全树注释二次复核验收（2026-08-14，轻量验收）

- **276 条 finding**：mount 56（11/18/27）、projection 65（16/22/27）、dir 74（27/24/23）、copyup 28（4/13/11）、security 25（1/5/19）、top_readdir 28（12/10/6）；处置合计 **delete 71 / simplify 92 / rewrite 113**。
- **标记类型**：REMAINING 59（第一轮执行缺口）/ NEW 99（第一轮漏网）/ ANNOTATED 118（AUDIT-* 待人工，二次复核确认全部维持原判断并给出方向）。
- **HIGH 2**：mount 1（术语/陈旧）、dir 1（whiteout accessor 文档与代码不符）。
- **关键结论**：第一轮清理“delete/simplify 执行了但 rewrite 只加标注未改写”；self-explain 漏网集中在 方法 doc 与行内/字段 doc 逐字重复、what-comment、N8 内联 Linux 引用。二次复核报告在 `components/wave9-comments-reaudit-20260814/`。

### 4.7 最终处置验收（2026-08-14，轻量验收）

- **276 条 finding 最终处置**：mount 11/23/21/1、projection 16/22/27/0、dir 30/23/21/0、copyup 4/15/9/0、security 3/7/15/0、top_readdir 12/11/5/0 → **delete 76 / simplify 101 / rewrite 98 / adjudicate 1**（闭合）。
- **ANNOTATED 不默认 rewrite 已落实**：4 条降 delete、9 条降 simplify；ADJUDICATE 5 条中 4 条已定，仅 mount F27/F28（无对应 finding 的标记）留 1 条 ADJUDICATE。
- **AUDIT-* 普查**：mount 25/27（F27/F28 单列）、dir 33/33（2 条额外可接受保留处置）、其余 scope 全覆盖。
- 6 份报告在 `components/wave9-comments-dispositions-20260814/`；可作为下一轮清理执行的唯一清单。

### 4.8 清理执行 Round 2 验收（2026-08-14，轻量验收）

- **执行计数**：mount 11/23/21/1、projection 16/22/27/0、dir 29/23/21/1（C6 skipped）、copyup 4/15/9/0、security 3/7/15/0、top_readdir 12/11/5/0 → delete 75 / simplify 101 / rewrite 98 / skipped 2（闭合 276）。
- **标记清零**：全树 `AUDIT-REWRITE`/`AUDIT-ADJUDICATE` = 0；USER_TODO/USE_TODO 57 行未动。
- **残留待办**：① mount R26（ADJUDICATE）保留；② dir C6 目标注释已不存在（仅 C4 幸存）；③ **dir W9 为代码级改动（删 accessor + 3 调用点改字段访问），未经编译验证** —— 下一轮必须先 `cargo check`（Checker 职责）。
- 收据在 `components/wave9-comments-cleanup2-20260814/`。

### 4.9 Reviewer Round 3 验收（2026-08-14，轻量验收）

- **291 条 finding**：mount 59（29/23/7）、projection 83（15/33/35）、dir 40（16/6/18）、copyup 46（11/15/20）、security 40（19/16/5）、top_readdir 23（9/8/6）→ **delete 99 / simplify 101 / rewrite 91**（闭合）。
- **C1–C6 分布**：C6 超长块是最大残留（copyup 20 / projection 34 / dir 18 / security 16 等），全部给了 rewrite 方向；C2/C3 自解释/解释型、C5 括号式 see 指针、C1 锁注释、C4 反向解释均有命中。
- **HIGH 11**：projection 10（含 user 点名项与超长块）、mount 1（`CreatorCredentialPolicy` doc 与 passthrough 现状矛盾）。
- **执行缺口确认**：前两轮 simplify/rewrite 残留未收敛共 12+ 处（如 mount R28、top_readdir 12 处）。
- 6 份报告在 `components/wave9-comments-reaudit3-20260814/`；作为下一轮清理的执行清单。

### 4.10 清理执行 Round 3 验收（2026-08-14，轻量验收）

- **执行计数**：mount 30/29/0/0、projection 15/68/0/0、dir 16/18/6/0、copyup 12/31/3/0、security 19/21/0/0、top_readdir 9/13/0/1 → **delete 101 / simplify 180 / rewrite 9 / trait_doc 1**（闭合 291）。
- **user 覆写落地**：91 条原 rewrite 绝大多数降为一句式 simplify 或 delete；仅 9 条模块级 doc 真正 rewrite（均 ≤8 行），readdir_at_impl 作为 trait-impl doc 保留一句行为描述。
- **残留待办**：① dir 模块 doc 的 Linux References / M1 helper 链接因 ≤8 行约束有取舍（收据）；② security xattr.rs #21 benign-double-evaluation 句随 C1 删除、语义保留于 `check_real_permission` doc（需确认）；③ **dir W9 代码级改动仍待 `cargo check`**；④ USER_TODO 57 行仍在（后续另案处理）。
- 收据在 `components/wave9-comments-cleanup3-20260814/`。

### 4.11 Reviewer Round 4 验收（2026-08-14，轻量验收）

- **52 条 finding（12 delete / 40 simplify / 0 rewrite，无 HIGH）**：mount 2（1/1）、projection 19（3/16）、dir 14（5/9）、copyup 9（2/7）、security 3（1/2）、top_readdir 5（0/5）。
- **收敛结论**：三轮清理后全树注释高度收敛；C2–C5 基本清零；剩余以 LOW 为主（N4 首行格式、C1 锁散点、P2 单点重复、C6 边界 9 行块、References 行号区间、1 处 C5 执行缺口）。
- **执行缺口 1 处**：copyup promote.rs:186-187 `(see the module References)`（R3 收据声称已删但仍在）。
- 6 份报告在 `components/wave9-comments-reaudit4-20260814/`。

### 4.12 清理执行 Round 4 验收（2026-08-14，轻量验收）

- **执行计数**：mount 1/1、projection 4/15、dir 5/9、copyup 2/7、security 1/2、top_readdir 0/5 → **delete 13 / simplify 39（52 全闭）**。
- **执行缺口补齐**：copyup promote.rs:186-187 C5 指针已删；References 行号区间补全（ovl_set_attr → copy_up.c 更正）。
- **代理挂起处理**：copyup 首派（Nash）约 25 分钟无产出/无收据，关闭后重派（Socrates）完成。
- **状态**：注释阶段暂告一段落（非定稿）；`AUDIT-*` 0 残留；USER_TODO/USE_TODO 57 行保留待另案。
- 收据在 `components/wave9-comments-cleanup4-20260814/`。

### 4.13 Reviewer Round 5（诚实轮）验收（2026-08-14，轻量验收）

- **24 条 finding（2 delete / 21 simplify / 1 rewrite，无 HIGH）**：mount 1、projection 7、dir 6、copyup 2、security 8、top_readdir 0（CLEAN）。
- **REGRESSION 4 条（诚实轮价值点）**：projection identity.rs:78（C5，R4 漏检）；security 3（R3 声称闭合的模块 doc↔方法 doc 重复仍在，R4 漏检）。
- **交主代理裁决**：security 4 条模块 doc 概述段（9-11 行）C6 阈值边界。
- **环境故障**：沙箱运行器二进制缺失（codex-linux-sandbox ENOENT），审计/写报告经用户批准非沙箱完成；无代码改动。
- 6 份报告在 `components/wave9-comments-reaudit5-20260814/`。

### 4.14 修复轮验收（Cleanup Round 5，2026-08-14，轻量验收）

- **执行计数**：mount 1/0/0、projection 0/7/0、dir 0/5/1、copyup 0/2/0、security 1/7/0、top_readdir 0/0/0 → **delete 2 / simplify 21 / rewrite 1（24 全闭）**。
- **REGRESSION 闭合**：identity.rs:78 C5、security Pipeline/DAC/is_private 重复全部真删，grep 0 命中；C5 括号指针全树 0。
- **C6 边界项**：security 4 条模块 doc 概述段 + Ownership gate 全部 ≤8 行，N2 保留。
- **状态**：注释阶段修复轮完成；编译仍待代码清理彻底后。AUDIT-* 0；USER_TODO/USE_TODO 57 行待另案。
- 收据在 `components/wave9-comments-cleanup5-20260814/`。

### 4.15 Reviewer Round 6 验收（2026-08-14，轻量验收）

- **9 条 finding（0/9/0，全 LOW，无 HIGH/MED，无 REGRESSION）**：mount 1、projection 2、dir 2、copyup 1、security 2、top_readdir 1。
- **修复轮闭合声明全部属实**（独立 grep 复核）：无回潮；历史“声称闭合实则仍在”模式未再复现。
- **交主代理裁决**：copyup promote.rs:179-181（R5 认可变体对 callee doc 复述，LOW）；security xattr.rs 两个 N2 块（copy_eligible_xattrs 9 行 / refresh_impure_marker 14 行）C6 阈值（simplify 保 N2 或明示 N2 例外）；top_readdir remove_visible doc C4/N2（future-optimization 从句）。
- 6 份报告在 `components/wave9-comments-reaudit6-20260814/`。

### 4.16 清理执行 Round 6 验收（2026-08-14，轻量验收）

- **simplified 9 / skipped 0 全闭**：mount 1、projection 2、dir 2、copyup 1、security 2、top_readdir 1。
- **三处裁决项落地**：copyup promote.rs:179-181（复述删、留标签）、security 2 个 N2 块（≤8 行、N2 全保留）、top_readdir remove_visible（C4 从句删、N2 句内保留）。
- **状态**：注释清理至此高度收敛（R6 为 9 条全 LOW，且全部修复）；AUDIT-* 0；USER_TODO/USE_TODO 57 行待另案；未提交。
- 收据在 `components/wave9-comments-cleanup6-20260814/`。

### 4.17 USER_TODO 另案（mount）+ mount 注释再审查（2026-08-15，V1 直派双任务）

- **user 指令**：mount 模块仍留 USER_TODO（绝大多数代码相关、少数针对注释）→ ① 派 Creator 一一找出、把问题
  记录在一处并删除标记；② 再派 Reviewer 按 handoff 21 条原则审查 mount 注释、找出不合规内容。用户指示使用
  **v1 multi-agent**（Direct Spawn Lane：`fork_context=false` + 自包含 spawn message，无需 user dispatch turn）。
- **Task A（Creator）`task_creator_mount_usertodo_record_20260815`（ACCEPTED）**：
  - 盘点 mount 树 7 文件当前工作树 `USER_TODO`/`USE_TODO` **40 条**（build 8 / claims 5 / layers 10 / mod 1 /
    options 2 / policy 9 / superblock 5；含 `USE_TODO` 拼写 build.rs:114），与 packet 基线 0 出入。
  - 全部记录入 `components/wave9-usertodo-mount-20260815/task_creator_mount_usertodo_record_20260815_record.md`
    （MT-01…MT-40：ID / 删除前 file:line / verbatim 原文含续行 / 提出的问题 / 分类：**代码相关 32 / 注释相关 8**）。
  - 标记块全部删除（40 块 / 75 行 + 悬空分隔行清理）；`grep USER_TODO|USE_TODO mount/` = **0**。
  - 主代理独立验收：逐 hunk 复核，7 文件删除全部为标记块 + 续行 + 悬空分隔行；build.rs/claims.rs/layers.rs 相对
    HEAD 的其余 diff 均属 user 未提交基线编辑（行数核算与 user 预派前 stat 吻合——build.rs 34 行变更含
    construction-local / Self-referential construction / WORKDIR_MODE 详细 rationale 等注释删除，非 A 所为）。
    计数微差：A 报 build.rs 21 行，实为 22（17 标记行 + 5 分隔行），不阻塞。
- **Task B（Reviewer）`task_reviewer_mount_comments_audit_20260815`（ACCEPTED）**：
  - 对 mount 树当前工作树（标记删除后）全部注释按 21 条原则 + C1–C6 + Trait-impl 一致 + PR Reviewer 文档原则
    诚实独立全量复核：**findings=3，全为 N2 设计决策 rationale 缺口（restore/add，NEW）；delete/simplify/rewrite/REGRESSION 均 0**。
  - **M-AUDIT-1（MED）** build.rs:218 `Arc::new_cyclic` 自引用构造（先发布后回填 root_inode / 无强环）rationale 被
    user 基线编辑删除 → N2 缺口，建议 restore/add。
  - **M-AUDIT-2（MED）** claims.rs:31 `WORKDIR_MODE` 与 Linux `0o000` 的偏差 rationale + References 被压成一句事实
    描述 → N2 缺口，建议 restore/add。
  - **M-AUDIT-3（LOW）** claims.rs:34 `WORKDIR_CLEANUP_MAX_DEPTH` 深度上限行为契约（更深非空目录以 ENOTEMPTY 失败
    而非递归清空）rationale 被删 → N2 缺口，建议 restore/add。
  - 8 条注释相关 MT（MT-14/18/19/20/24/25/32/36）复核均不构成现行原则违反；MT-24（`with_current_posix_thread`
    why/where）现无 doc 但按 C2 自解释，保留为可选项待主代理裁决。
  - 无回潮：M-REAUDIT6-1（policy.rs TODO 与正文复述）cleanup6 修复核实未回潮；AUDIT-* 0；USER_TODO 0。
  - 报告：`components/wave9-mount-comments-audit-20260815/task_reviewer_mount_comments_audit_20260815_audit.md`。
- **user 裁决（2026-08-15）**：
  - **M-AUDIT-1（自引用构造）不补回**：`Arc::new_cyclic` 为 Rust core 库 API，名称/语义已自明，注释强行解释属
    越俎代庖 → 属 **C2 自解释即删 + P2 注释信息密度**（注释只补代码之外的信息；复述 std 库语义不增加信息）。
  - **M-AUDIT-2（WORKDIR_MODE Linux 偏差）不补回**：代码体注释不提 Linux 为基本原则 → 属
    **N8 References 替代内联 Linux 细节 + PR Reviewer 文档原则**（正文不复述 Linux 实现细节；偏差/对齐不以内联
    Linux 论述表达）。
  - **M-AUDIT-3（WORKDIR_CLEANUP_MAX_DEPTH）暂缓**：涉及代码逻辑（workdir 清理策略，MT-11 / §4.2 原 P9），
    注释随代码裁决后定 → 属 **§4.2 非原则的代码位置疑虑**（待裁决，非注释原则）。
  - 三项决策本身具有原则性：注释不为 std/代码已自明之事补解释（C2/P2）；不在代码体注释提 Linux（N8）；
    代码逻辑未定前不固化依赖逻辑的注释（§4.2）。
- **仍待裁决**：② build.rs 被删的 "A successful persist is a durable identity record and is never rolled back"
  （持久化不回滚契约）与 "Claim the upper slot first…"（claim 顺序 rationale）是否补回；③ layers.rs `RealPath`
  无 struct doc 是否补；④ MT-24 是否补一句 why doc。
- **Task C（Creator）`task_creator_nonmount_usertodo_record_20260815`（ACCEPTED，2026-08-15 续，user 指令「mount 之外的 user todo 也清理」）**：
  - mount 之外 overlayfs 源码树 USER_TODO **5 处**：`mod.rs:3-4`（可见性迁移过程性笔记，1 块 2 行）+
    `readdir_index.rs` `:64` / `:72-76`（含 2 条标记）/ `:102-103`（4 块，8 行）。
  - 全部记录入 `components/wave9-usertodo-nonmount-20260815/task_creator_nonmount_usertodo_record_20260815_record.md`
    （NM-01…NM-05：**注释相关 4 / 过程性笔记 1**）；标记块全部删除（10 行）；**overlayfs 源码树（.agents 外）USER_TODO = 0**。
  - 主代理独立验收：两文件 diff 仅标记块 + 悬空分隔行删除；`ReaddirIndexEntry` enum 块删后无孤立 `///`；
    `mod.rs` `#![short_vis_path::add(overlayfs)]` 上移，无其它改动。
  - 4 条注释相关（NM-02/03/04/05）+ 1 条过程性笔记（NM-01）的问题保留在记录中待裁决。
- **mount 外 USER_TODO 已清零（2026-08-15）**：源码树（`.agents/` 外）全树 `grep USER_TODO|USE_TODO` = 0。
- 未提交、未编译；工作树含 user 未提交基线编辑（build.rs/claims.rs/layers.rs）。

### 4.18 全量注释 Review Round（2026-08-15，user 指令「对所有 overlay comment 再做一次 review」）

- **dispatch**：6 个 Reviewer 子代理（V1 直派、并行、只读）：mount / projection / dir / copyup / security / top_readdir。
  共享 criteria `subagent-tasks/wave9-comments-fullaudit-20260815/wave9_comments_fullaudit_CRITERIA.md`（并入 user
  2026-08-15 裁决：C2/P2 强化、N8 强化、§4.2 延迟、M-AUDIT-1/2/3 不补报）。
- **结果：9 findings（delete 0 / simplify 8 / rewrite 1 / restore_add 0）；MED 1 / LOW 8；NEW 8 / REMAINING 1 / REGRESSION 0**。
  - **mount 2（LOW simplify）**：M-FA-1 layers.rs 行内注释与方法 doc 重复「构造期强制 lowerdir 非空」rationale（P2/C3）；
    M-FA-2 claims.rs:31 `WORKDIR_MODE` doc 复述代码可见常量值 0o700（C2/P2 强化）。
  - **projection 3（LOW simplify）**：PRJ-1 inode.rs `new_root` doc 括号句解释 `Arc::new_cyclic` std 自引用语义（C2/P2 强化，M-AUDIT-1 同类）；
    PRJ-2 inode.rs `fs` 字段 doc 与 struct doc 不变量重复（P2/C3，R3 N3 残留）；PRJ-3 identity.rs `allocate_fallback_ino` doc 走读
    `AtomicU64::try_update` std 契约（C2/P2 强化）。
  - **dir 1（LOW simplify）**：D-1 whiteout.rs:154 未解释缩写 `(BIO-capable)`（P2/P3）。
  - **security 3（N8 强化首轮应用，均 NEW）**：S-1 xattr.rs:227 `as Linux does`（MED simplify，Known-divergence N2 保留）；
    S-2 permission.rs:110 `completes the Linux shape`（LOW rewrite，fsgid 析取 rationale 保留）；S-3 metadata.rs:22/25 模块 doc
    `Linux requires…`/`Linux-faithful`（LOW 边界项 simplify——模块 doc 非严格「代码体」，交主代理裁决）。
  - **copyup 0 / top_readdir 0（CLEAN）**。
- **回潮/闭合复核**：0 REGRESSION。R6/cleanup1–6 闭合声明（identity.rs:78 C5、模块 doc↔方法 doc 重复、promote.rs:179-181/186-187、
  remove_visible C4/N2、security N2 块 ≤8 行）全部独立核实真闭合；M-AUDIT-1/2/3 与 NM-01…05 按裁决未报/不报。
- **主代理验收注记**：9 条 finding 内容均与当前代码文本核实一致；个别报告行号有漂移（如 projection inode.rs `new_root` doc
  实际 214-218 而非报告 131-132；mount layers.rs 行内注释实际 ~209-215 而非 199-202），执行轮须以当前文本重新锚定，不阻塞验收。
- 报告 6 份在 `components/wave9-comments-fullaudit-20260815/`。

- **Cleanup Round 7（ACCEPTED，2026-08-15）**：4 个 Creator（mount/projection/dir/security，V1 直派并行）执行
  §4.18 的 8 条 finding——**mount simplified=2、projection simplified=3、dir simplified=1、security simplified=1+rewritten=1**；
  **S-3（metadata.rs 模块 doc Linux 表述）按 user 裁决先留着（skipped=1）**。主代理独立验收：8 处改动逐处与代码核实
  （grep 目标短语全树 0 残留），N2 rationale 全保留、清单外未动、无代码/可见性改动；收据 4 份在
  `components/wave9-comments-cleanup7-20260815/`。至此全量 Review 9 条中 8 条闭合，**仅 S-3 待裁决**。

### 4.19 N8 放置专项审查（2026-08-15，user 指令「内部 doc 无需引用 Linux，只应放模块顶层 doc」）

- **背景**：user 指出内部 doc 引用 Linux 无必要（点名 layers.rs:144/182/212），历轮审计（含 fullaudit Round 7）漏报——
  前几轮按**强化前** N8（vt/manager 方法级 References 为模式）判内部 References 合规；user 强化后规则为
  **仅模块顶层 `## References` 区可放 Linux 引用**。
- **dispatch**：1 个 Reviewer（Hume，V1 直派、只读）grep 普查全树 `elixir.bootlin|Linux|ovl_` →
  **72 行命中 / 21 文件**（0 命中 10 文件；与 packet 基线 20 文件差 1，如实报告）。
- **结果：hits=72 → 顶层 OK 34 / 内部 FINDING 34（27 逻辑块）/ 边界 4；MED 6（=user 点名 layers.rs 三项）/ LOW 28；REGRESSION 0（全部 NEW——规则变更后重分类）**。
  - **内部 34 行**：方法/函数 doc 17（build.rs 2、layers.rs 4、xattr.rs 3、permission.rs 1、remove.rs 4、whiteout.rs 2、link.rs 1）、
    常量 doc 4（xattr.rs 3、whiteout.rs 1）、行内 12（options.rs 2、layers.rs 2、metadata.rs 5、rename.rs 3）、方法 doc 正文 1（entry.rs:211）。
  - **处置**：move to 模块顶层 References **26 行/19 块**；delete **8 行/8 块**（xattr.rs 常量 3、metadata.rs 行内 4、entry.rs:211）。
  - **边界 4 行（交 user 定口径：模块 doc 正文能否出现 Linux 字样）**：metadata.rs:22/25（S-3，已挂起）、entry.rs:12（模块 doc 正文
    "Linux overlayfs merge-stop semantics"，同 doc References 有链接）、lower_id.rs:11（模块 doc 正文 "not Linux-wire-compatible"，N2 偏差 rationale）。
  - 已知必报项 ML-1/2/3（layers.rs:144-145/182-183/212-213）均在列，当前文本锚定。
- **主代理验收**：逐项抽查（build.rs:266、options.rs:143、xattr.rs:102、metadata.rs:65、entry.rs:12/211、lower_id.rs:11）与代码一致；
  计数自洽（34/27/26/8/4）。报告：`components/wave9-n8linux-audit-20260815/task_reviewer_comments_n8linux_audit_20260815_audit.md`。
- **去重提示**：同源链接多文件重复（overlayfs.h#L42-L54、attr.c#L161-L226、overlayfs.rst#L350-L364、groups.c#L227-L237），
  move 时每模块 References 保留单条；options.rs/permission.rs/remove.rs/whiteout.rs/rename.rs/link.rs 现无模块 References 区，需新建。

- **Cleanup Round（ACCEPTED，2026-08-15）**：4 个 Creator（mount/security/dir/projection，V1 直派并行）按 user 裁决
  （引用与 Linux 偏差可在 doc 存在，其它 Linux 出现一律清理）执行——**moved=19、deleted=7、reworded=4、kept=1**：
  - mount：moved=5（build/options/layers 内部 References → 模块顶层）；security：moved=5、deleted=7（xattr 常量 3 +
    metadata 行内 4，同链接已在模块 References）、reworded=2（metadata.rs:22/25 去 Linux 字样）；dir：moved=9（remove/
    whiteout/rename/link 新建模块 References 区）；projection：reworded=2（entry.rs:12/211 去 Linux 字样）、kept=1
    （lower_id.rs:11 偏差 rationale 保留）。
  - 主代理独立验收：全树剩余 Linux/elixir/ovl_ 命中 70 处**全部在 `//!` 模块 doc 行**（顶层 References + lower_id.rs:11
    偏差）；内部（非 `//!`）Linux 命中 = **0**；options/permission/remove/whiteout/rename/link 新建 References 区；
    4 处 reword 语义保留；无代码/可见性改动；收据 4 份在 `components/wave9-n8linux-cleanup-20260815/`。

### 4.21 PR #3708 Reviewer 反馈——原则并入记录（2026-08-15，user 确认并入，不 append）

PR #3708 reviewers（cchanging / tatetian / zjp-CN）反馈已并入 §4.1（2026-08-15，user 确认「需要并入，复述/补充
注意不要 append」）：**复述/补充的并入原条目，真正新增的 N12 入列表**。

- PR-N1（TODO 格式）→ **N12 新增**（§4.1.1 第 14 条；代码原则序号顺移为 15–22）。
- PR-N2（注释术语准确一致）→ **并入 P10**（第 22 条）。
- PR-N3（禁跨文件 file:line 引用）→ **并入 P3**（第 2 条）。
- PR-N4（doc 高层 summary + 禁推理倾泻）→ **并入 N4**（第 6 条）。
- PR-N5（术语先定义后使用/开篇核心术语/不枚举目录）→ **并入 N9**（第 11 条）。
- PR-N6（doc-as-spec 精度）→ **并入 N5**（第 7 条）。
- PR-N7（注释附着语义承载处）→ **并入 P5**（第 17 条）。
- 已覆盖确认：cchanging 大段 review（N8/N9/N10/N11）、zjp-CN short-vis-path（P1）未新增。
- 代码建议（删死代码/重组模块/去缓存/锁重构/封装/单测）本轮不处理。

### 4.22 PR #3708 原则审计 Round（2026-08-15，6 个 Reviewer 并行，聚焦 7 项并入原则）

- **dispatch**：6 个 Reviewer（mount/projection/dir/copyup/security/top_readdir，V1 直派并行、只读），criteria
  `subagent-tasks/wave9-pr3708-principles-audit-20260815/`；聚焦 N12/P3/N4/N9/N5/P5/P10 并入后判据，其余原则为基础。
- **结果：33 findings（delete 2 / simplify 11 / rewrite 16 / add 6，含少量 rewrite+add 双处置）；REGRESSION 8（判据强化
  后重违，文本未回退）/ NEW 25**。
  - **mount 11**（P3×4、N4×4、N12×1、N5×1、P5×1）：layers.rs:53-54/59、policy.rs:239 跨文件路径；superblock.rs:138、
    claims.rs:93 首行非 summary；options.rs:19-30 `MountOptionKey` variant doc（P5 明文示例）；options.rs:84-87 `parse` doc
    缺空值/切分边界（N5）；1 处 `/// TODO` 嵌 doc（N12）。
  - **projection 3**：**F1 N9 HIGH——projection/mod.rs 模块 doc 未开篇定义 "projection"/"binding"，以枚举目录为主
    （tatetian 点名处，判定不通过）**；F2 N12 lower_id.rs:264 `/// TODO(origin-verify):` 嵌 doc；F3 N5
    `is_whiteout_inode` 校验入口无 doc/行为边界。
  - **dir 2**：link.rs:6 P3 跨文件 `dir/mod.rs`；remove.rs:375 N12 `/// TODO(stale-upper):` 嵌 doc。
  - **copyup 3**：mod.rs:9、coordination.rs:11 P3 跨模块文件路径（REGRESSION）；coordination.rs:5 P2 自指路径冗余。
  - **security 9**：P3×7（xattr.rs:101-103/110-112、permission.rs:44/102 等跨模块路径，模块 doc 定位句 LOW 边界）；
    N9×2（metadata.rs/xattr.rs "admission"/"creator-credential scope" 未在 doc 内定义）。
  - **top_readdir 5**：P3×1（readdir_index.rs `projection/inode.rs`，REGRESSION）、N9×2（模块 doc 开篇枚举 +
    Tombstone/opaque layer 术语未先定义）、N4×1（首行非动词引导）、N5×1（readdir_at_impl spec 边界）。
  - **N12 合规样例**：permission.rs:136-139 独立 `// TODO:`（doc 外+退出条件）合规。
- **主代理验收**：逐项抽查（projection/mod.rs F1、options.rs MountOptionKey、remove.rs:375、copyup/mod.rs:9、
  lower_id.rs:264）与代码一致；计数自洽（个别报告 header 计数笔误已按报告正文修正，如 projection add=1）。
- 报告 6 份在 `components/wave9-pr3708-principles-audit-20260815/`。

### 4.23 PR #3708 原则审计 33 findings——处理方案（2026-08-15 记录，user 指示「暂不执行」）

user 询问处理方案后指示：**先记录到 handoff，暂不执行**。本方案为既定计划，执行待另行指令。

**Step 0 — 边界裁决（记录时采用建议，执行前可再确认）**
1. **P5×1（`MountOptionKey` variant doc）**：**暂缓至代码清理轮**——与 cchanging 删冗余抽象建议绑定；届时注释随代码移到
   match 分支（P5 意图）。本轮不动。
2. **P3 模块 doc 定位句（security 3 处 LOW）**：不豁免，统一「**去路径、留语义**」（如 `in `mount/build.rs`` →
   "at mount build time"）。
3. **copyup 兄弟文件枚举 / N9 模块结构表**：保留——N9 允许结构表枚举子模块；P3 只禁正文/括号跨文件路径引用。

**Step 1 — Cleanup 轮（6 个 Creator 并行，按 scope；未执行）**
- P3×12（rewrite/simplify）：跨文件路径 → rustdoc 链接或语义描述。
- N12×3（rewrite）：`/// TODO(x):` 移出 doc → doc 后独立 `// TODO: ...` 行（保留退出条件）。
- N4×5（rewrite）：首行 → 动词引导高层 summary。
- N9×5（rewrite/add）：模块 doc 补核心术语定义——**projection/mod.rs 开篇重写（重点，tatetian 点名）**补
  "projection"/"binding" 定义；readdir_index 补 Tombstone/opaque layer；metadata/xattr 补 "admission"。
- N5×3（add）：补行为边界 doc（`parse` 空值/切分/重复 key、`is_whiteout_inode` 双路径+错误语义、`readdir_at_impl`）。
- P2×1（simplify）：copyup coordination.rs:5 自指路径冗余。

**Step 2 — 复核**：cleanup 后一个 Reviewer 轮（或主代理逐条 grep + 抽查）验证 33 条闭合、无新引入（重点：N9 重写真定义
术语、N5 补 doc 达 spec）。

**Step 3 — amend**：验收后 amend 到 wave-9 WIP 提交。

### 4.24 A/B 灵活性原则全量注释审查（2026-08-15，user 指令「对 overlay 全文现存的所有 comments，针对 AB 两档灵活性原则，再次全面审查」）

- **背景**：user 担忧此前执行审查的模型（V4 Flash）难以处理灵活性判据；本 wave 只审
  A/B 两档（A：N1/N2/C2/C3/P2/N10/N11；B：N9/N5/C4/C1/P10/P5/P9′），C/D 档不作正式判据。
- **dispatch**：6 个 Reviewer 子代理（V1 直派、并行、只读）：mount / projection / dir / copyup /
  security / top_readdir。共享 criteria `subagent-tasks/wave9-comments-abflex-audit-20260815/
  wave9_comments_abflex_audit_CRITERIA.md`；packets 同组目录。
- **关键口径**：① 不以求收敛；历轮结论非豁免依据。② PR3708 报告中属 A/B 判据的 finding
  （N9/N5/P5/P2，未执行清理）独立复核后标 `REMAINING` 再报；C/D 档不正式报。③ fullaudit
  cleanup7 的 A/B 档 finding 做 REGRESSION 回潮复核。④ user 裁决 M-AUDIT-1/2/3、§4.2 延迟、
  S-3（N8）不报。⑤ P9′/P5 代码级项只标 `adjudicate`，不改代码。
- **输出**：6 份只读审计报告 → `components/wave9-comments-abflex-audit-20260815/`。
- **验收（2026-08-15，main agent 逐条核实）**：6/6 报告收齐、结构合规；13 条 finding 全部与当前
  代码文本逐条锚定核实；`git status` 无任何 `.rs` 改动（仅 3 个 `.agents` 管理文件），写集合规。
  **cleanup 未授权，执行待 user 指令。**
- **结果汇总（13 findings）**：按原则 **N9=6 / N5=4 / P5=1 / P2=1 / C1=1**（其余 A/B 原则 0）；
  按处置 **delete=1 / simplify=2 / rewrite=7 / restore_add=2 / adjudicate=1**；
  按标记 **REMAINING=10 / NEW=3 / REGRESSION=0**；按严重度 **HIGH=1 / MED=2 / LOW=10**。
  - **REMAINING=10**：恰为 PR3708 中 A/B 档全部 10 条（mount P5/N5、projection N9/N5、copyup P2、
    security N9×2、top_readdir N9×2+N5），文本均未变，复核后按 REMAINING 再报。
    top_readdir 两条 `restore_add` 为“补写 doc”语义（TR-AB-2/3，原 PR3708 F3/F5），非 N2 误删恢复。
  - **NEW=3**：mount layers.rs:57-60 C1 锁作用域注释（delete）；projection entry.rs
    `is_opaque_directory` doc N5 spec 缺口（rewrite）；dir link.rs 模块 doc 使用 `DIR` 未定义（N9 rewrite）。
  - **fullaudit cleanup7 的 A/B 档 finding 全部闭合**（mount M-FA-1/2、projection PRJ-1/2/3、dir D-1），
    0 REGRESSION；M-AUDIT-1/2/3、§4.2、S-3 排除项均未报。
  - **P9′**：无 adjudicate（各 scope 未发现注释仅为一行式 accessor 背书）；M-AB-1 为 P5 附着问题，
    随代码清理 adjudicate。
  - C/D 档提醒（不计入本 wave finding）：PR3708 的 P3/N12/N4 项仍在原处，后续按 §4.23 处理。

#### 4.24.1 新轮 findings 机械/灵活两档分类（2026-08-15，user 指示；执行前口径）

对 §4.24 全部 13 条 finding 按**执行形态**分两档（不是重新裁定，而是为后续 Creator 派发定口径）：

- **机械档（3 条）**：判定已完成，执行 = 锚定文本删除或插入模板句，无语义设计。可交弱执行者或由
  main agent 直接做。
  1. **M-AB-3**（mount layers.rs:57-60，C1，delete）：删已锚定的锁作用域注释块。
  2. **CP-AB-1**（copyup coordination.rs:5，P2，simplify）：删自指路径 `` `copyup/coordination.rs` ``，
     改 “the coordination surface”。
  3. **D-AB-1**（dir link.rs:3-10，N9，rewrite）：插入与 dir 兄弟模块一致的 lock-contract 句
     （目标文案可由 main agent 冻结，如 “the caller holds the parent `DIR`; this module enters `CUL`
     only via source promotion”）。
- **灵活档（10 条）**：执行需读代码语义、起草/改写文档文本（术语定义、模块 doc 重写、doc-as-spec），
  不适合交给弱模型独立做。分两小类：
  - **文档设计类（5 条）**：PRJ-AB-1（projection/mod.rs 开篇定义 projection/binding，HIGH）、
    S-AB-1（metadata.rs 定义 admission/creator-credential scope）、S-AB-2（xattr.rs 定义 admission gate）、
    TR-AB-1（readdir_index 模块 doc 去枚举、写高层语义）、TR-AB-2（定义 Tombstone/opaque layer）。
  - **doc-as-spec 类（4 条）**：M-AB-2（`parse` 行为边界）、PRJ-AB-2（`is_whiteout_inode`）、
    PRJ-AB-3（`is_opaque_directory`）、TR-AB-3（`readdir_at_impl`）。
  - **代码耦合类（1 条）**：M-AB-1（`MountOptionKey` P5 附着，adjudicate，与删冗余抽象的代码清理绑定）。
- **降级通道**：灵活档若由 main agent 先起草逐条目标文案（尤其 doc-as-spec 4 条与 D 类术语定义），
  则可降级为机械执行；M-AB-1 除外，必须随代码清理轮处理。
- **执行建议**：机械档 3 条可先单独执行并验收；灵活档建议等 user 对目标文案口径（术语定义放置位置、
  spec 精度）确认后派较强 Reviewer/Creator 执行。本分类不改变“cleanup 未授权、执行待 user 指令”。

#### 4.24.2 执行派发（2026-08-15，user 指令：Flash 执行机械档；双 Pro 辩论灵活档出方案）

- **Lane 1 机械档**：1 个 **deepseek-v4-flash Creator**
  `task_creator_abflex_mechanical_cleanup_20260815` 按 packet 精确 old/new 执行
  M-AB-3/CP-AB-1/D-AB-1（write-set：mount/layers.rs、copyup/coordination.rs、dir/link.rs
  指定注释 + 收据）。
- **Lane 2 灵活档**：2 个 **deepseek-v4-pro** 按
  `subagent-tasks/wave9-comments-abflex-exec-20260815/flexible_plan_debate_BRIEF.md`
  对 10 条做 A 提案 → B 批驳 → A 终稿；产物
  `components/wave9-comments-abflex-exec-20260815/flexible_plan_final.md`（及其过程稿）。
  **不修改 `.rs`；终稿不实施。**
- 状态：**已执行并验收（2026-08-15）**。
  - Lane 1：Flash 机械档 3/3 完成；diff 与 packet old/new 精确一致，0 代码行改动；收据
    `components/wave9-comments-abflex-exec-20260815/task_creator_abflex_mechanical_cleanup_20260815_receipt.md`。
  - Lane 2：双 Pro 辩论完成（A 提案 / B 批驳 6 修正 4 同意 / A 终稿）；终稿 10/10 覆盖、
    N2 保留、无越权改码；`flexible_plan_final.md` 含主代理验收注记（自检计数校正 4 处）。
    **user 已批准终稿并派发执行（2026-08-15）**：4 个 deepseek-v4-flash Creator 按
    `flexible_plan_exec_SPEC.md` 分 scope 并行执行（mount / projection / security /
    top_readdir；10 处 exact old→new）。
  - **灵活档执行验收（2026-08-15，main agent 逐 hunk 核对）**：4/4 收据在册；10/10
    （M-AB-1/2、PRJ-AB-1/2/3、S-AB-1/2、TR-AB-1/2/3）替换与 SPEC 逐字一致；全树 9 个
    `.rs` diff 仅注释行（`git diff --unified=0` 非注释 +/- 行 = 0）；0 代码行改动；
    未编译、未提交。
  - M-AB-1 终稿口径（parse doc 唯一承载 + 删 6 条 variant doc + match 分支不加注释，
    避免三重复制）**已获 user 批准并执行**。

### 4.25 顶层 `//!` 模块 doc 专项审查（2026-08-15，user 指令「参考 book/ 下 guide 审查顶层 doc 是否足够好」）

- **前置**：ABFLEX 全部改动已 `git commit --amend --no-edit` 并入 wave-9 WIP（新 HEAD
  `f91ab8407`，37 files +1965/−4570，工作树干净）。
- **dispatch**：1 个 Reviewer（只读）
  `task_reviewer_overlayfs_topdocs_quality_20260815`；scope = overlayfs 全树 `//!` 模块 doc
  （32 个 `.rs`，legacy/.agents 除外）；判据 = book guides
  `for-maintainability/comments.md`、`rust-specific/comments.md`、
  `rust-specific/crates-and-modules.md#module-docs`、`for-documentation/general-style.md`；
  handoff §4.1 为背景。
- **执行（2026-08-15，user 指令）**：F1 顶层 doc 参考 ext2/exfat 写法补 `//!`；P3 7 条（F3–F9）
  执行。1 个 Creator `task_creator_overlayfs_topdoc_p3_fix_20260815` 按
  `subagent-tasks/wave9-topdocs-fix-20260815/topdocs_p3_fix_SPEC.md` 执行 8 处 exact old→new。
  **执行验收（2026-08-15，main agent 逐 hunk 核对）**：8/8 与 SPEC 逐字一致；8 个 `.rs`
  diff 仅注释行；收据
  `components/wave9-topdocs-fix-20260815/task_creator_overlayfs_topdoc_p3_fix_20260815_receipt.md`。
- **F2/F10/F11 已执行（2026-08-15，main agent 直接注释级修复）**：
  F2 `dir/whiteout.rs` 模块 doc 补 `WhiteoutCache`/`WhiteoutHandle`/`WhiteoutRepresentation` 清单；
  F10 `readdir_index.rs` 删除 “head section below” 实现指针；
  F11 `readdir_index.rs` 概述段去掉与 `## Index contract` 重复的 cookie 细节。
- **锁域术语清查（2026-08-15，user 指令，已验收）**：Reviewer
  `task_reviewer_overlayfs_lock_vocab_20260815` 报告 24 findings
  （HIGH 3 / MED 21；define 6 / rewrite 16 / delete 2；no_op 6）。main agent 已独立核实
  HIGH 3 与 user 点名项：
  - `readdir_index.rs:22-23` `UPPER`/`MOUNT` 无对应代码锁，应删除/改写（HIGH）；
  - `copyup/mod.rs:14-16` Lock contract 漏记 `record_copyup_transition` 的 `CUL` `try_lock`（HIGH）；
  - `dir/whiteout.rs:9-10` `WL` 无任何定义、`DIR` 仅依赖未指向的 `dir/mod.rs` 定义（MED）；
  - `dir/mod.rs:6` 全局 Lock vocabulary 缺 `WL` 定义（MED）；
  - `BIO` 在 copyup/coordination.rs 未展开（REMAINING）。
  报告：`components/wave9-lock-vocab-review-20260815/..._audit.md`。
  **执行验收（2026-08-15，main agent 逐 hunk 核对）**：Creator
  `task_creator_overlayfs_lock_vocab_rewrite_20260815` 按 SPEC 完成 36/36；
  全树注释锁缩写（`DIR`/`CUL`/`INODE`/`WL`/`UPPER`/`MOUNT`/`BIO`）残留 grep=0；
  新增文本无 `snapshot`；13 个 `.rs` diff 仅注释行；收据
  `components/wave9-lock-vocab-fix-20260815/task_creator_overlayfs_lock_vocab_rewrite_20260815_receipt.md`。
  未编译、未提交。
  - **勘误**：`BIO` 在仓库内语义为 **block I/O**（`aster_block` 的 `Bio` 请求/设备 I/O），
    不是 blocking I/O；coordination.rs 终稿已由 main agent 修正为 “block I/O”。
- **审查验收（2026-08-15）**：报告
  `components/wave9-topdocs-review-20260815/task_reviewer_overlayfs_topdocs_quality_20260815_audit.md`；
  **11 findings（HIGH 0 / MED 5 / LOW 6）**，main agent 逐条锚定核实并已全部执行
  （F1 顶层 doc、F2 类型清单、F3–F9 P3×7、F10/F11）。

### 4.26 user 全局审阅 projection 标注归纳（2026-08-16，user 指令）

- **背景**：user 在 projection 7 文件工作树写下 53 条审阅标注——**31 条 `USER_COMMENTS`**
  （注释疑问）+ **22 条 `USER_CODES`**（代码疑问）；指示派 1 个 deepseek-v4-flash 子代理
  归纳两类、`USER_CODES` 单独记录留待后续、`USER_COMMENTS` 判定违反哪条现有 principle 并
  解释以前为何没发现；完成后仅把 projection 复原到 `git HEAD`（user 同期在审 copyup，其它
  模块不得复原）。
- **dispatch**：`task_reviewer_projection_user_annotations_20260816`（Reviewer、只读、Low），
  packet `subagent-tasks/wave9-projection-user-annotations-20260816/
  task_reviewer_projection_user_annotations_20260816_dispatch.md`；经 workflow 直派，
  `provider: opencode-go` + `model: deepseek-v4-flash`（首次 `provider: deepseek` 子代理失败，
  更正 provider 后重跑成功；本次限定模型的正确键名记录于此备查）。
- **结果（ACCEPTED 2026-08-16，main agent 结构验收）**：
  - `components/wave9-projection-user-annotations-20260816/user_codes_summary_20260816.md`
    —— UC-01…UC-22 全部覆盖，每条 file:line + 原文 + 归纳，结论统一「留待后续处理」，
    按 cache/命名/errno/职责/可读性等分组。
  - `components/wave9-projection-user-annotations-20260816/user_comments_principle_analysis_20260816.md`
    —— CM-01…CM-31 全部覆盖，每条判定违反的现行原则（最多为 C2/P2/N5/N9/N4 强化类）、
    给出依据与「以前为何没发现」归因（曾处理但残留 / 强化判据重判 / §4.23 已记录未执行）。
- **复原**：仅 projection 7 文件 `git restore` 到 HEAD；projection `USER_COMMENTS|USER_CODES`
  grep = 0。**copyup/mod.rs 未动**——user 同期新增 7 条标注（3 COMMENTS + 4 CODES）仍保留，
  待 user 读完该模块后另行指示。
- **待办**：31 条 USER_COMMENTS 判定的清理执行、22 条 USER_CODES 代码级处置均未授权；
  copyup 当前标注尚不处理。

### 4.27 全树按本轮判据再审查（2026-08-16，user 指令「派出六个 reviewer，按此轮发现的问题再审查所有 overlayfs 代码」）

- **dispatch**：6 个 Reviewer 子代理并行（workflow 直派，`provider: opencode-go` +
  `model: deepseek-v4-pro`），scope = mount（`overlayfs/mod.rs` + `mount/*`）/ projection /
  dir / copyup / security / top_readdir；**只读、只写报告**。
- **判据**：`subagent-tasks/wave9-principles-fullaudit-20260816/wave9_principles_fullaudit_CRITERIA.md`
  ——把 §4.26 的 CM-01…31 归纳为 7 组模式：复述/自解释（P2/C2/C3）、叙述质量（N4/N1/C4）、
  文档边界（N5）、模块 doc 与术语（N9/P10/P3/N8）、rationale 边界（N2/P4）、锁注释（C1）、
  C6 阈值。审计对象一律为 **git HEAD（`a24ce3e70`）**，不读工作树 user 标注；projection 的
  CM-01…31 为已知项不重复报。
- **结果（ACCEPTED 2026-08-16，main agent 结构验收：6/6 报告收齐、计数闭合、锚点抽查属实、
  0 处 `.rs` 改动）——109 findings（NEW 96 / REMAINING 13 / REGRESSION 0；
  HIGH 1 / MED 15 / LOW 93）**：

  | scope | findings | NEW/REM/REG | HIGH/MED/LOW | 处置 |
  |---|---|---|---|---|
  | mount | 29 | 22/7/0 | 0/8/21 | delete 4 / simplify 13 / rewrite 11 / adjudicate 1 |
  | projection（CM 之外） | 33 | 32/1/0 | 0/3/30 | delete 15 / simplify 13 / rewrite 5 |
  | dir | 8 | 7/1/0 | 1/0/7 | delete 1 / simplify 4 / rewrite 3 |
  | copyup | 16 | 16/0/0 | 0/1/15 | delete 1 / simplify 10 / rewrite 5 |
  | security | 16 | 12/4/0 | 0/3/13 | delete 1 / simplify 8 / rewrite 7 |
  | top_readdir | 7 | 7/0/0 | 0/0/7 | delete 1 / simplify 5 / rewrite 1 |
  | **合计** | **109** | **96/13/0** | **1/15/93** | **delete 23 / simplify 53 / rewrite 32 / adjudicate 1** |

- 主要命中模式：C4 反向解释、C2 自解释 doc、P2 复述、N5 实现细节、P3 跨文件路径、
  N9/N4 模块 doc 质量、N12 嵌 doc TODO；REMAINING 13 条全部来自 §4.23「暂不执行」的
  PR #3708 C/D 档存量（mount 7、projection 1、dir 1、security 4），非回潮。
- 报告 6 份在 `components/wave9-principles-fullaudit-20260816/`；packets 同
  `subagent-tasks/wave9-principles-fullaudit-20260816/`。
- **执行未授权**：109 条清理 + 22 条 UC 代码处置均待 user 指令。
- **工作树注意**：审计期间 user 继续在 copyup 写标注，现为 3 文件共 10 条
  （`mod.rs` 7 + `promote.rs` 2 + `workdir.rs` 1；USER_COMMENTS 3 / USER_CODES 7），
  **全部未动**，待 user 读完 copyup 后另案。

### 4.28 copyup user 标注归纳 + 与 fullaudit 对比（2026-08-16，user 指令）

- **dispatch**：1 个 deepseek-v4-flash Reviewer（workflow 直派，`provider: opencode-go` +
  `model: deepseek-v4-flash`）`task_reviewer_copyup_user_annotations_20260816`；只读，只写两份报告。
- **scope**：copyup 工作树 user 标注 **10 条（3 `USER_COMMENTS` + 7 `USER_CODES`）**；
  数量与主代理盘点一致（`mod.rs` 7 + `promote.rs` 2 + `workdir.rs` 1）。
- **结果（ACCEPTED 2026-08-16）**：
  - `components/wave9-copyup-user-annotations-20260816/user_codes_summary_copyup_20260816.md`
    —— UC-01…UC-07 全部归纳、全部「留待后续处理」（命名 1 / 职责位置 3 / 设计机制 2 /
    rationale 缺失 1）。
  - `components/wave9-copyup-user-annotations-20260816/user_comments_vs_fullaudit_copyup_20260816.md`
    —— 3 条 COMMENT 与 COPYUP-FULL-01…16 逐条对比：
    - COM-1（Lock contract 只应提 copy-up lock，O_APPEND 锁叙述多余）→ **RETAIN**，
      C1(+P2)，simplify 收窄；fullaudit 无同 target 覆盖。
    - COM-2（“## Per-call delegation”段含义不明）→ **COVERED-BY COPYUP-FULL-01**
      （同句 C4、同 rewrite 方向），按 user 口径不保留为待办。
    - COM-3（`workdir_temp_serial` doc 中 “lifecycle”/“gates” 难懂）→ **RETAIN**，
      N4/P10，rewrite；fullaudit 未覆盖。
  - 结论：COMMENTS 3 条 → retained 2 / covered 1；与 §4.27 的 copyup 16 条合并后，
    copyup 注释待办仍以 fullaudit 16 + retained 2 为准（COM-2 不重复计入）。
- **未执行/未删除**：7 条 UC 与 retained 2 条处置待 user 指令；copyup 工作树标注**仍在**，
  未授权 `.rs` 复原。

### 4.29 两轮清理执行（2026-08-16，user 指令：R1 Flash delete+simplify；R2 Pro 提案→核准→rewrite）

- **R1（ACCEPTED）**：6 个 deepseek-v4-flash Creator 并行执行 95 项 delete/simplify
  （fullaudit 76 + CM delete/simplify 18 + COM-1 1）——6/6 收据在
  `components/wave9-principles-cleanup-r1-20260816/`，95/95 done、0 skipped。
  28 个 `.rs` diff 仅注释行；未编译；已 amend 入最新 wave-9 WIP。
- **R2 提案（main agent 已核准）**：6 个 deepseek-v4-pro Creator 先产 46 条 rewrite 提案
  （fullaudit 32 + CM rewrite 13 + COM-3 1），`proposed=46 blocked=0`；main agent 修订 5 处：
  RIDX-FULL-02 `Serve`→`Serves`（N4 第三人称单数）；PROJECTION-FULL-04 首行回归
  “Creates or reuses”（不虚构 publish）；CM-19 去掉 `OverlayObjectFacts` “immutable”；
  CM-31 简化 per-inode guard 措辞；MOUNT-FULL-29 保留 lower 并发修改 N2 bullet、仅删
  `refresh_impure_marker` 跨模块指称；MOUNT-FULL-19 bullet 保持 `-`。
  提案在 `components/wave9-principles-cleanup-r2-20260816/proposals/`；
  main-agent 修订在 `subagent-tasks/wave9-principles-cleanup-r2-20260816/
  main_agent_approval_r2_*.md`。
- **R2 执行（ACCEPTED）**：6 个 deepseek-v4-pro Creator 并行执行，46/46 done、0 skipped；
  收据在 `components/wave9-principles-cleanup-r2-20260816/`。
  验收：所有 old-phrase grep 0 残留；`/// TODO` 全树 0 残留；`git diff --check` clean；
  28 个 `.rs` diff 仅注释行；未编译；已 amend 入最新 wave-9 WIP。
- **本轮闭合口径**：141 条 actionable 注释 finding 全闭（CM 31 + fullaudit 108 + copyup
  retained 2；fullaudit 的 1 条 adjudicate = mount `RealPath` struct doc 仍挂起）。
  USER_CODES（projection 22 + copyup 7）与其它代码级裁决仍留待后续。

### 4.30 metadata_security user 标注整理（2026-08-17，user 指令「先整理它们」）

- **dispatch**：1 个 deepseek-v4-flash Reviewer（workflow 直派，`provider: opencode-go` +
  `model: deepseek-v4-flash`）`task_reviewer_metadata_user_annotations_20260816`；只读，
  只写两份报告。
- **scope**：metadata_security 4 文件工作树 user 标注 **16 条（12 `USER_COMMENTS` +
  4 `USER_CODES`）**，与主代理盘点一致；工作树标注未删除。
- **结果（ACCEPTED 2026-08-17，结构验收）**：
  - `components/wave9-metadata-user-annotations-20260816/
    user_comments_summary_metadata_security_20260816.md`——12/12 逐条归纳
    （file:line + 原话 + 所指注释现状 + 疑问 + 模式分类 + 初步关联原则，明确不裁决）。
  - `components/wave9-metadata-user-annotations-20260816/
    user_codes_summary_metadata_security_20260816.md`——4/4 代码级疑问归纳，
    全部「留待后续处理」。
- **主要问题模式**：模块 doc 术语未定义/结构不清（gate、uid/fsuid、kernel contexts fail
  open、Ownership gate 概念顺序）、重复句与空泛词（cross-cutting）、`<T>` rationale 缺失、
  N2 rationale 可读性差（refresh_impure_marker）、自解释/复述式常量与方法 doc、
  Known divergence 状态不清。
- **待办**：与最新 fullaudit/R1-R2 清理结果的对比、裁决与清理执行均待 user 指令；
  4 条 USER_CODES 代码级疑问留待后续。

### 4.31 全树按 metadata 新问题模式再审查（2026-08-17，user 更正：六个模块都要审）

- **dispatch**：metadata_security 已先单独完成（§4.30 之后，
  `task_reviewer_metadata_principles_audit_20260817`，1 个 deepseek-v4-pro Reviewer）；
  其余五个 scope（mount / projection / dir / copyup / top_readdir）按同一 M1–M9 判据
  并行补派 5 个 deepseek-v4-pro Reviewer。共享判据
  `subagent-tasks/wave9-allmodules-principles-audit-20260817/
  wave9_allmodules_principles_audit_CRITERIA.md`。
- **口径**：审计 git HEAD（`f5d4975e0`）；只读、只写报告；2026-08-16 R1/R2 已闭合项
  不重复报，除非回潮；`RealPath` adjudicate 与 USER_CODES 不判定。
- **结果（ACCEPTED 2026-08-17，结构验收；六 scope 合计 70 findings：
  NEW 58 / REMAINING 12 / REGRESSION 0；HIGH 1 / MED 14 / LOW 55）**：

  | scope | findings | NEW/REM/REG | HIGH/MED/LOW | 报告目录 |
  |---|---|---|---|---|
  | metadata_security | 18 | 6/12/0 | 1/9/8 | `components/wave9-metadata-principles-audit-20260817/` |
  | mount | 12 | 12/0/0 | 0/2/10 | `components/wave9-allmodules-principles-audit-20260817/` |
  | projection | 12 | 12/0/0 | 0/0/12 | 同上 |
  | dir | 12 | 12/0/0 | 0/3/9 | 同上 |
  | copyup | 5 | 5/0/0 | 0/0/5 | 同上 |
  | top_readdir | 11 | 11/0/0 | 0/0/11 | 同上 |
  | **合计** | **70** | **58/12/0** | **1/14/55** | |

- 主要模式命中：M1 未定义术语（gate/admission/payload-less/cross-cutting 等）最多，
  其次 M3 重复句、M5 长句链、M6 自解释 doc、M7 状态不清、M8 复述实现、M9/C6 超长 N2
  段落；0 REGRESSION（历史闭合项 grep 复核无回潮）。
- **执行未授权**：70 条 finding 的清理与 metadata 工作树标注的删除/复原均待 user 指令。

### 4.32 全 Flash 两阶段清理（2026-08-17，user 指令「这次全交给 flash，不要用 pro」）

- **R1 Flash delete+simplify（ACCEPTED）**：6 个 deepseek-v4-flash Creator 并行执行
  **36 项**（metadata 7 / mount 6 / projection 5 / dir 8 / copyup 3 / top_readdir 7），
  36/36 done、0 skipped；收据在
  `components/wave9-flash-cleanup-20260817/r1/`；diff 仅注释行。
- **R2 Flash 提案（ACCEPTED，main agent 核准）**：6 个 deepseek-v4-flash Creator 先产
  **34 条 rewrite/add 提案**（metadata 11 / mount 6 / projection 7 / dir 4 / copyup 2 /
  top_readdir 4；top_readdir 1 条因 R1 已部分修改而标 BLOCKED，由 main agent 补文）。
  main-agent 修订 4 处：MOUNT-M-AUDIT-01 不改方法名（`apply_capability_gates` 保留，
  只改注释）；DIR-M-AUDIT-07 去 `DIR` 锁缩写；COPYUP-M-AUDIT-04 首行改动词引导
  “Maps the copy-up phase values…”；READDIR-M-AUDIT-07 用 R1 后文本补批。
- **R2 Flash 执行（ACCEPTED）**：6 个 deepseek-v4-flash Creator 并行执行，
  **34/34 done、0 skipped**；收据在
  `components/wave9-flash-cleanup-20260817/r2/`。
  验收：`git diff --unified=0` 非注释 `.rs` 行 = 0；关键 old-phrase grep 0 残留；
  `apply_capability_gates` 未被改名；未编译、未提交。
- **本轮闭合口径**：70 findings 的 actionable 注释项全部闭合（R1 36 + R2 34）；
  metadata 4 条 USER_CODES 与 projection/copyup 代码级 UC 仍留待后续。
- **工作树注意**：改动未 amend；元数据标注由 user 自行重置，无残留。

### 4.33 day-2 全 Flash 注释复审（2026-08-17，user 指令：amend 后再派 flash 对每个模块按
昨天+今天新问题审查，且**只审查注释**）

- **amend**：R1/R2 Flash 清理与管理记录已 amend 入最新 wave-9 WIP（`bfeedeb69`，工作树曾干净）。
- **dispatch**：6 个 deepseek-v4-flash Reviewer 并行，scope = metadata / mount / projection /
  dir / copyup / top_readdir；共享判据
  `subagent-tasks/wave9-day2-flash-audit-20260817/wave9_day2_flash_audit_CRITERIA.md`
  （§1 昨日 P2/C2/C3/N4/N5/N9/P10/P3/N8/N12/C1/C6 模式 + §2 今日 M1–M9 模式；
  **只审注释/文档，代码不报**；已闭合项不重复报，除非回潮）。
- **结果（ACCEPTED 2026-08-17，结构验收；六 scope 合计 54 findings：
  NEW 54 / REMAINING 0 / REGRESSION 0；HIGH 0 / MED 11 / LOW 43）**：

  | scope | findings | 严重度 |
  |---|---|---|
  | metadata | 8 | 0/2/6 |
  | mount | 3 | 0/0/3 |
  | projection | 25 | 0/3/22 |
  | dir | 12 | 0/6/6 |
  | copyup | 4 | 0/0/4 |
  | top_readdir | 2 | 0/0/2 |
  | **合计** | **54** | **0/11/43** |

- 主要命中：M6 一行式 accessor/常量 doc（projection 最多）、M1 未定义术语、M3 重复句、
  M8 复述实现、C1/C6 边界；0 REGRESSION。
- 报告 6 份在 `components/wave9-day2-flash-audit-20260817/`；packets/criteria 同
  `subagent-tasks/wave9-day2-flash-audit-20260817/`。
- **执行未授权**：54 条 finding 的清理待 user 指令；代码问题本轮未审。

### 4.34 day-2 六 packet 指针 + Flash 清理中断记录（2026-08-17）

- **六个 day-2 注释审计 packet（本 tenure 第一类 packet 指针）**：
  `subagent-tasks/wave9-day2-flash-audit-20260817/`
  下 6 个 `task_reviewer_day2_flash_audit_<scope>_20260817_dispatch.md`
  （scope = metadata / mount / projection / dir / copyup / top_readdir）
  与共享判据 `wave9_day2_flash_audit_CRITERIA.md`；6 份报告在
  `components/wave9-day2-flash-audit-20260817/`（共 54 findings，见 §4.33）。
- **Flash 清理中断**：user 确认上游 `opencode-go` 5h 限额触发，本轮 Flash 清理暂停。
  `subagent-tasks/wave9-day2-flash-cleanup-20260817/{r1,r2}/` 中 R1/R2 清单与 dispatch
  packet 已备好（R1 delete+simplify 34 项、R2 rewrite 20 项含 delete/simplify 降档许可），
  待限额恢复后续派；当前工作树仅 `readdir_index.rs` 有 R1 的 2 处 partial 注释改动
  （READDIR-D2-01/02，diff 已核实为注释行、方向正确）。
- **第二类 packet（代码结构清理）指针不在本 handoff**：见新 handoff
  `main-agent/20260817-code-structure-cleanup_main_agent_handoff.md`。

### 4.35 day-2 Flash 清理恢复执行（2026-08-17，user 指令「恢复 day-2 的 flash 注释清理」；R1+R2 全闭）

- **R1（ACCEPTED）**：5 个 deepseek-v4-flash Creator（workflow 直派，`provider: opencode-go` +
  `model: deepseek-v4-flash`）执行 delete+simplify——**done 34 / skipped 0 / total 34**：
  copyup 2 / dir 4 / metadata 4 / projection 22 / top_readdir 2（上次中断前 partial 改动，
  本轮 verify 后记 `done(pre-applied)`，未重改）；mount 0 条不派。5 份收据在
  `components/wave9-day2-flash-cleanup-20260817/r1/`。
  （workflow 调用在结果序列化时报错一次，但 5/5 收据与 `.rs` 改动均已落盘；主代理按产物
  验收，未重派。）
  - 主代理独立验收：全部 `.rs` diff 非注释行 = 0；34 条逐条对照 SPEC 闭合；
    main agent 直接修正 copyup trigger.rs 一处语法（`is release` → `is released`，
    仅注释行）。
  - projection R1 SPEC 的「计数闭合」小节原为 audit 全量 25 条口径（含 3 条 rewrite），
    main agent 已更正为本 R1 执行口径 22 条；top_readdir SPEC 增补恢复执行注记。
- **R2 提案（ACCEPTED）**：5 个 deepseek-v4-flash Creator 先产提案（只写报告、不改 `.rs`），
  **proposed=20 blocked=0 total=20**（mount 3 / projection 3 / dir 8 / copyup 2 /
  metadata 4；top_readdir 0 不派）。提案在
  `components/wave9-day2-flash-cleanup-20260817/r2/proposals/`。
- **R2 核准（main agent）**：20/20 核准；修订 2 处——
  ① DIR-D2-07「the rmdir type/emptiness gates」→「the type and rmdir-emptiness gates」
  （type gate 属 unlink、emptiness gate 属 rmdir）；② PROJ-D2-16「no resolution is
  needed」→「this caller skips the layer resolution」（`project_object_id_from_lower_id`
  内部仍会 resolve，注释只描述本调用点跳过）。核准文件在
  `subagent-tasks/wave9-day2-flash-cleanup-20260817/r2/main_agent_approval_day2_flash_r2_*.md`。
- **R2 执行（ACCEPTED）**：5 个 deepseek-v4-flash Creator 并行执行——**done 20 / skipped 0 /
  total 20**。5 份收据在 `components/wave9-day2-flash-cleanup-20260817/r2/`。
  主代理独立验收：20 条 old-phrase 在目标文件 grep 0 残留（permission.rs 模块 doc 的
  “admission”与代码日志字符串中的 “impure-marker”不在本清单，未动）；全部 `.rs` diff
  非注释行 = 0；`git diff --check` clean；C6 目标块均 ≤8 行；未编译；已 amend 入最新
  wave-9 WIP。
- **day-2 54 findings 全闭**：R1 34 + R2 20 = 54/54，0 skipped、0 REGRESSION。
  本轮未触碰代码结构清理 packet；已随 wave-9 WIP amend。

## 5. Next Actions for the Next Thread (CRITICAL)

0. **user 全局审阅（进行中）**：user 逐模块审阅所有代码（含注释成果）；注释阶段非定稿。进度：projection 53 条
   标注已归纳归档并复原（见 §4.26，31 CM 原则分析 + 22 UC 待处理）；copyup 10 条标注已归纳归档
   （见 §4.28：CODES 7 留待后续；COMMENTS 3 条 → retained 2 / covered 1，**工作树标注仍在，
   复原待 user 指令**）；**metadata_security 16 条标注（12 COMMENTS + 4 CODES）已整理归档
   （见 §4.30，工作树标注仍在，对比/裁决/复原待 user 指令）**；其余模块待 user 继续审阅。
   如审出问题再开确认/清理轮。
1. **USER_TODO/USE_TODO**：源码树（`.agents/` 外）已**全树清零**（2026-08-15：mount 40 条 + mount 外 5 条均记录+删除，见 §4.17）。
   问题记录两处：mount `components/wave9-usertodo-mount-20260815/task_creator_mount_usertodo_record_20260815_record.md`
   （32 条「代码相关」裁决清单）；mount 外 `components/wave9-usertodo-nonmount-20260815/task_creator_nonmount_usertodo_record_20260815_record.md`
   （NM-01…NM-05：4 条注释相关 + 1 条过程性笔记待裁决）。
2. **N2 缺口/边界项裁决（§4.17）**：M-AUDIT-1/2 已裁决**不补回**（C2/P2；N8）；M-AUDIT-3 **暂缓**待代码逻辑
   （workdir 清理策略，§4.2 / MT-11）。仍待裁决：persist 不回滚契约 / claim 顺序 rationale / `RealPath` struct doc
   是否补回；MT-24 是否补一句 why doc。
3. **裁决 §4.2 遗留疑虑**：workdir 清理策略（原 P9，代码位置疑虑，已记录为 MT-11）对照 Linux 行为后给出结论。
3b. **全量注释 Review 9 条 finding**：8 条已执行并验收（cleanup7，见 §4.18）；S-3 并入 N8 边界口径待裁决。
3c. **N8 放置专项（§4.19）**：已按 user 裁决执行完毕（引用/偏差保留、其它 Linux 清理；moved=19/deleted=7/reworded=4/kept=1，
   验收通过）；全树内部 Linux 命中 0，仅模块顶层 References 与 lower_id.rs:11 偏差 rationale 保留。
3d. **PR #3708 原则并入（§4.21）**：7 条已并入 §4.1（N12 新增；P3/N4/N5/N9/P5/P10 补充）。
3e. **PR #3708 原则审计 33 findings（§4.22 已验收；处理方案 §4.23 已记录，暂不执行）**：P3×12 / N9×5 / N12×3 /
   N4×5 / N5×3 / P5×1（暂缓至代码清理轮）/ P2×1。执行待 user 指令（先走 §4.23 Step 0 边界裁决 → Step 1 cleanup →
   Step 2 复核 → Step 3 amend）。
3f. **A/B 灵活性原则全量审查（§4.24，2026-08-15 已派发并验收）**：13 findings（N9×6/N5×4/P5×1/P2×1/C1×1；
   10 REMAINING=PR3708 A/B 档、3 NEW；0 REGRESSION）。
3g. **两档执行（§4.24.2，2026-08-15 已执行并验收）**：Flash 机械档 3/3 + 灵活档 10/10
    （4 个 Flash Creator 按 SPEC 执行），9 个 `.rs` diff 仅注释行、0 代码改动；未编译未提交。
    灵活档已全部落地，无 pending。
3h. **顶层 `//!` 模块 doc 专项审查（§4.25，2026-08-15 已验收并执行）**：11 findings 全部执行
    （F1 顶层 doc、F2 类型清单、F3–F9 P3×7、F10/F11 readdir_index 两条）。
3i. **锁域术语清查与去缩写（§4.25，2026-08-15 已验收并执行）**：24 findings 经
    36 处 exact old/new 全部闭环；全树注释锁缩写残留=0，新文本无 snapshot。
4. **代码清理阶段**：待 user 指令进入代码清理（含 dir W9 accessor 代码改动、可见性短路径全树迁移），彻底完成后统一编译验证（Checker：`cargo check -p asterinas --target x86_64-unknown-none` + `make check` + `make docs`）。
4a. **day-2 Flash 清理已完成（§4.35）**：54/54 全闭、未编译；已 amend 入最新 wave-9 WIP；
    编译验证仍待代码清理彻底后统一执行。
5. **push/PR 决策**：分支未 push（与 origin diverged）；最终定稿后决定 force-push 与 PR 分支同步。

## 6. Live File Discipline

- **This file is the live handoff for:** wave-9 注释审计 tenure（2026-08-13 起）。
- **Update rule:** 本文件原地更新直至下一 tenure；逐条裁决、迁移完成、编译验证等新结论都记入本文件或由其指向的新 handoff。
- **Supersedes / Replaces:** `20260813-rebase-upstream_main_agent_handoff.md`（rebase/API-repair/验证 tenure 已 CLOSED；其内容作为历史记录保留，仍有效）。
