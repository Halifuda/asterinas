<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-30 单测与新回归测试规划（开题）

**Date / Time:** 2026-08-30 20:30 CST
**Status:** `OPENED — 仅开题。测试内容尚未动工；范围与载体待 user 裁决后再切片（Designer → Creator/Reviewer 既有执行模型）。`
**Parent:** 前一 tenure handoff
`20260826-140940-parent-copyup-state-design_main_agent_handoff.md`
（**CLOSED** 2026-08-30；wave 五 pass 闭环 + make check GREEN + xfstests
22 例单次运行 + regression 两例单测，遗留 gap 清单见其「HANDOFF CLOSED」节。）

## 1. 使命（开题陈述）

为刚落地的 copyup 收敛（pass_50/51）与 xattr 整体重构（pass_55）的行为面
建立**本仓自有的测试资产**，分两条线：

- **新回归测试**：`test/initramfs/src/regression/fs/overlayfs/` 下新增
  C 用例（沿用既有两例 `ovl_test` / `readdir_small_buffer` 的自包含风格：
  程序内自建 lower/upper/work、自挂 overlay、断言后自清理）。
- **单测**：载体待决（见 §3 open question ①）。

**当前状态：仅开题。** 本文只记录候选内容与待决点；未经 user 指令不派发
任何 packet、不改任何测试源。

## 2. 候选回归用例（草案，待 user 圈定）

对齐本轮新行为面（对应上游不可调度用例的本地可运行版）：

1. **转义语义回归**（E1/E2/E3；对应上游 026 现代语义与 084 意图）：经
   overlay 对 own-prefix 名 `trusted.overlay.fsz` set → 盘上加段
   `trusted.overlay.overlay.fsz`（以底层直读验证）→ get 往返取回 →
   list 呈现剥回一段 → 未转义 own 记录（opaque/whiteout/impure/origin）
   对 list/get 隐藏；`trusted.overlayfsrz` 类前缀边界名原样透传。
2. **userxattr 挂载回归**（U1/U2/U3）：`-o userxattr` 挂载成功；标记与
   origin 记录落在 `user.overlay.*`；能力探针按命名空间选择。
3. **嵌套叠加分层**（E3/I1 段数=层数）：overlay 叠 overlay，各层标记
   各自命中、互不误伤、对上层用户不可见。
4. **readdir `..` 与 d_ino 恒等**（recorded_parent 直读、D-51 后形态）：
   038 主题的本地轻量断言版。
5. （待裁）**022 修复合入后**的 overlay-as-upperdir 拒绝用例——依赖
   022 修复（前一 handoff gap 1），先挂起。

## 3. Open questions（user 裁决项）

1. **单测载体**：协议铁律禁 overlayfs 任何 ktest 面（PROTOCOL rule 17）。
   "单测"的载体需要决策：(a) 维持禁令、只做回归线；(b) 放宽允许 OSDK
   ktest 形式的 overlayfs 单测（需正式修订协议）；(c) 其它载体（如纯逻辑
   下沉到可 `cargo test` 的非 OSDK crate——涉及代码搬移，需设计）。
   候选纯逻辑单测点（若载体获批）：`classify`/`used_full_name`/
   `present_xattr_names`（纯函数）、`XinoMode` 编码、`RealObjectKey`
   层序规则。
2. **回归用例清单冻结**：§2 五项哪些进本轮、断言口径、是否连同
   `fs/run_test.sh` 注册与 runlist/block.list 联动。
3. **测试源改动归属**：新用例落在 `test/initramfs/`（仓库跟踪面），
   需明确本线允许触碰该目录的写集边界（此前 overlayfs wave 从未改过）。

## 4. Next-main-agent actions

1. 将 §2/§3 提交 user 裁决：单测载体、回归用例圈定、写集边界。
2. 裁决后按既有执行模型切片（bounded Designer 冻结面 → 每 pass
   Creator + Reviewer → per-pass commit）。
3. 动工前铁律自检：无 ktest 面（除非 ① 明式放宽并修订协议）、
   无 overlayfs 之外的生产改动（除测试目录授权写集）。

## 5. Prohibitions（直至 user 指令）

不派发任何测试相关 packet；不改 `test/` 任何文件；不改生产代码；
不运行任何测试（xfstests/regression/ktest 均未授权）。
