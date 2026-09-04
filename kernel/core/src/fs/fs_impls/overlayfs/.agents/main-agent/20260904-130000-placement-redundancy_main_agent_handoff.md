<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-09-04 代码位置与冗余处理轮（开题）

**Date / Time:** 2026-09-04 13:00 CST
**Status:** `OPEN — 仅开题。user 裁决（2026-09-04）：UuidMode::Null 的问题单独开一轮「代码位置与冗余处理」；未经 user 指令不派发任何 packet、不改任何生产代码。`
**Parent:** 前一 tenure handoff
`20260830-203000-unit-and-new-regression-tests_main_agent_handoff.md`
（**CLOSED** 2026-09-04；测试线 + mount v2 + 注释线两轮全闭环，见其
§6-§21。）

## 1. 使命（开题陈述）

处理注释线两轮的顺带设计发现并延伸的**代码位置与冗余**问题：

1. **UuidMode::Null 归属（本轮触发点，user 点名）**：`fs/policy.rs` 的
   `UuidMode::Null` 与 `Off` 在全树所有消费点均成对处理
   （`Off|Null` 一起匹配），两模式无任何行为差别。三个候选方向：
   (a) 合并——删除 `Null` 变体，parse/verify 把 `uuid=null` 归入
   `Off`（语义上最诚实：null 就是"不持久化"）；
   (b) 赋实义——给 `Null` 区分行为（如持久化全零记录 vs 不持久化），
   需先对 `/home/ayd/linux` 上游核实 Linux 是否有对应模式；
   (c) 保留现状但记录理由（如为未来兼容预留）——需 N2 rationale
   有真实依据，否则与"冗余处理"目标矛盾。
   设计事实源：`components/mount-options-v2-20260831/` 的 v2 设计
   （MO1 键矩阵冻结了 `uuid` 三态），任何改动须回读该 spec 并走
   bounded Designer 修订，不允许 Creator 直接改。
2. **冗余处理延伸（候选，待 user 圈定）**：注释线 review 轮暴露的两处
   code-as-spec 裁决（marker-read errno 语义由 `has_marker` 函数体
   承载、real-object invariant 由 debug_assert 承载）与
   `Off|Null` 同类——**同一语义多处近重复表达**（代码臂/断言/doc/
   冻结测试表）是否需要收敛。若圈入本轮，先做全树小规模普查再裁。

## 2. Open questions（user 裁决项）

1. §1.1 的三选一（合并 / 赋实义 / 保留现状）。
2. §1.2 的延伸普查是否圈入本轮，还是本轮只做 Uuid 单点。
3. 执行模型：单点改动走「Designer bounded 修订 → Creator → Reviewer」
   既有模型，还是按 2026-08-30 惯例 Creator 带 compile_preflight 一次
   闭环。

## 3. Next-main-agent actions

1. 将 §1/§2 提交 user 裁决。
2. 裁决后：Uuid 改动前先核上游（`/home/ayd/linux` v7.2.0-rc3 树 +
   mount-options-v2 spec）并出 bounded Designer 修订，再切片执行。
3. 动工前自检：改动面限于 `fs/policy.rs`/`fs/mount/{options,policy
   消费点}.rs`（census 后定）；注释线两轮的成果（一句话/两句上限）
   适用于本轮所有新注释。

## 4. Prohibitions（直至 user 指令）

不派发任何 packet；不改任何生产代码；不运行任何测试/编译。

## 5. 排队项（自前一 handoff 顺延，次序待 user 指令）

G1 凭据轮（R-1 复活门槛）、GAP-KTEST-001 归属裁定、R-3
characterization + flock 修复 pass、Inode trait 重构 PR pick、
ktest 静默控制台缺口归属、022 修复后的 overlay-as-upperdir 回归用例。

## 6. 2026-09-04 开工记录：全 enum 分支冗余审计（user 指令）

- **Dispatch**：`task_audit_enum_variants_20260904`（只读 Reviewer，
  Direct Spawn Lane；packet 在 `subagent-tasks/placement-redundancy-20260904/`）。
- **Census**：19 enum / 56 变体，全部有构造点（DEAD = 0）；ktest 块内
  无 enum 定义。
- **Verdict**：DISTINCT 17 / **INSEPARABLE 2**：
  1. `UuidMode::Off|Null`（fs/policy.rs:17-18，锚定案）：3 处消费点
     （capabilities.rs:149、inuse.rs:147、mount/mod.rs:154）全部配对，
     另 mount/mod.rs:71 单挑 On；唯一差异是 derive 与 ktest 断言。
  2. `NegativeLookup::Absent|HiddenByOpaque`（user 点名深挖）：9 个
     消费点永远同臂（create.rs:38 显式配对，其余 8 处落 `_`/补集）；
     `HiddenByWhiteout` 则在 4 处被独立对待，挣得存在权——**enum 整体
     不冗余**。附：`NegativeBinding` 已不在 active 树（仅存归档）。
  合并两者均机械可行（NegativeLookup 单生产者 lookup_in_layers）。
- **冗余候选处置提案**（报告 `components/placement-redundancy-20260904/
  enum_variant_audit_20260904.md`，待 user 裁决）：
  1. `UuidMode::Off|Null` → merge / keep-with-rationale /
     needs-upstream-check 三案；报告倾向先核 Linux 上游再 merge。
  2. `NegativeLookup::Absent|HiddenByOpaque` → merge / keep-with-
     rationale 两案；报告倾向 keep（配对编码"上层 opaque 屏障 vs 全扫描
     耗尽"溯源 + create 臂镜像 Linux 语义）。
- **Next**：两候选的取舍待 user 裁决；裁 merge 侧先走 bounded Designer
  修订（回读 mount-options-v2 spec），再 Creator。

## 7. 2026-09-04 追记：Uuid 上游核查结果（主代理直查，建议翻案）

- **User 裁决（本日）**：① Uuid 按"先核上游再做"放行；②
  `NegativeLookup` 倾向**删 `HiddenByOpaque` 并入 `Absent`**（user 理由：
  parent opaque 即整个 overlay 视图不可见，无任何上游操作可触达，两形态
  无观测差；user 并确认此前已核过 Linux 无区分需求）。
- **上游核查（/home/ayd/linux v7.2.0-rc3，file:line 实证）**：
  `uuid=` 为四态 off/null/auto/on（params.c:72-76、默认 auto :85-87）——
  本地 `UuidMode` 四变体是对上游表面的忠实镜像，**Null 非本地发明**。
  OFF≠NULL 的分叉点唯一：`ovl_uuid_match`（namei.c:164-172）的
  origin-fh 校验规则由 `ovl_origin_uuid`（`uuid != OFF`）选择；且 NULL
  是上游通用降级靶（params.c:901-903、util.c:811-816/842、super.c:741）。
  本地 `Off|Null` 同行为的原因 = 分叉点属未实现的 index/export 特性族；
  本地 parse（options.rs:192-195）与 Auto 解析（mount/mod.rs:142-154，
  镜像 util.c）均已就位。
- **主代理建议翻案（原倾向 merge）**：keep `Null` + 变体上一句 doc
  锚定上游分叉点。理由：① merge 需 parse 归一化 `uuid=null` 且
  show_options 回显偏离上游；② 上游降级靶是 NULL 非 OFF，将来
  index/origin-fh 落地时该变体即降级靶；③ 与 MO5 的 parity 忠实先例
  一致。**待 user 裁决**。
- **User 终裁（本日，§7 两案落地）**：① `UuidMode::Null` **保留**（面向
  用户的分支面），加一句注释解释 Off/Null 并存原因——主代理翻案采纳；
  ② `NegativeLookup` 删 `HiddenByOpaque` 并入 `Absent`（user 确认此前已
  核 Linux 无区分需求）。
- **Dispatch（Direct Spawn Lane）**：`task_designer_placement_redundancy_20260904`
  （bounded Designer 修订；packet 在
  `subagent-tasks/placement-redundancy-20260904/`；两个 deliverable =
  Uuid doc 句冻结 + HiddenByOpaque 删除冻结（含 lookup/dir 文档连带面），
  上游锚点表已随 packet 下发、要求 Designer 复核后再冻结；产物 =
  designer_spec.md + designer_validation.md）。
- **Next**：Designer 产物结构验收 → 执行轮切片（Creator compile
  preflight = `cargo osdk check -p aster-core`）。

## 8. 2026-09-04 执行记录：两修订闭环

- **Designer 验收**：`task_designer_placement_redundancy_20260904` 结构
  验收 ACCEPTED——两 deliverable 冻结面齐备；上游逐行复核无内容不符
  （树内无 tag，按内容核验；copy_up.c:463-464、util.c:841 两处行号漂移
  已录）；D1 文案单句、三点上游支撑全对上；D2 不变量 I1（gate 保留）
  已冻结。计数 slip 一处（validation 文档 "six lines" vs spec 冻结 5 行）
  以 spec 为准，Creator 已正确处理。
- **Creator 执行**：`task_creator_placement_redundancy_20260904` —
  4 文件写集全部逐字落地：D1 policy.rs +5 行注释（`UuidMode::Null` doc）；
  D2 lookup.rs 变体删除 + 生产臂改 `Absent`（I1 gate 保留）+ 模块 doc
  重写（"upper-miss" 退役）+ create.rs 配对臂收拢 + dir/mod.rs
  key-concept 行更新；其余 8 个消费点按 per-site 表不动；import 零变化。
  **compile gate exit 0**（codex-asterinas-dev `cargo osdk check -p
  aster-core`，11.88s；容器 Exited(137) 被 `docker start` 唤起后原样执行
  packeted 命令，已记收据）。census：删一个 enum 变体（56→55），零新
  生产实体。`HiddenByOpaque` 活跃树 grep 零残留。
- **主代理验收**：exact-diff 逐行核对 = 冻结面吻合，**ACCEPTED**。
- **状态**：4 文件改动未提交（+handoff/PASS_SLICING/BLUEPRINT 记账），
  commit 待 user 指令。Round 两项 user 点名工作全部闭环；§1.2 的
  code-as-spec 延伸普查未被圈入（维持 review 轮裁决），如需扩展待
  user 指令。
- **Next**：commit（待指令）；排队项不变（G1、GAP-KTEST-001、R-3+flock、
  Inode trait pick、ktest 静默控制台归属）。

## 9. 2026-09-04 追记：rustdoc 与 make check lint 门（user 指令）

- **rustdoc**：`cargo osdk doc -p aster-core --no-deps` exit 0 零警告
  （D1 变体 doc 与 D2 doc 重写均无 intra-doc link 问题）。
- **make check**：首轮 **FAIL** 于 rustfmt 门——14 处漂移集中在
  options.rs/identity.rs/xattr.rs 的 ktest 测试块（`present_expect_ok`
  等 assert 调用折行）。归因：**§13 时代遗留债**（当时 5 处换行修复
  注明"需 make check 最终确认"，此后 0ab6442f6/287b1ea5d 等提交均未跑
  全量 check；上次 GREEN 2026-08-30 run04 早于 ktest 测试入库），非本轮
  改动引入。机械 `cargo fmt --all` 修复（3 文件，纯调用点折行）→
  `cargo fmt --all -- --check` clean → **make check 重跑 exit 0 全绿**
  （完整日志容器 /tmp/make_check_run2.log）。
- **状态**：工作树新增 3 文件机械 fmt 修复（options.rs/identity.rs/
  xattr.rs，代码行折行，非注释），与本轮 4 文件修订 + 记账一并未提交，
  commit 待 user 指令。
