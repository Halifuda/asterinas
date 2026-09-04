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
