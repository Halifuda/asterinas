<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-09-05 PR 进程开启

**Date / Time:** 2026-09-05 19:34 CST
**Status:** `OPEN — PR 进程开启。分支已推送（force-with-lease，58b346df5）。
本 handoff 为 overlayfs 多代理工作区的最后一份 in-tree 记录：随后 .agents
整体归档并移出工作树（/home/ayd/overlayfs-agents/），后续记录在归档地
续写；skill（ovfs-main/ovfs-subagent/ovfs-checker）已改指归档地。`
**Parent:** `20260905-pr3767-adaptation-execution_main_agent_handoff.md`
（适配 + runtime gates + 022 + ktest 各轮，全部 CLOSED）；更早的 PR 计划
先例 = `20260812-pr-draft-prep_main_agent_handoff.md`。

## 1. 分支状态快照（PR 的交付物现状）

`codex/overlayfs-refactor-new` = `origin/main`（`864b9138b`）+ 29 个重放
提交（overlayfs 新实现全部波次）+ PR #3767 两笔 pick（dentry-centric
trait，ccchanging 署名保留）+ 适配执行波（P1 rename PoC / P2a admission+
frame / P2b namespace+plumbing，R1+R2 双 PASS）/ P3 Variant B 简化 +
runtime-gate 修复 + 022 修复 + 记账。**有效全表面 xfstests 21/21**；
回归 4/4；make check 全绿（唯一残留 nixfmt = 环境漂移，文件与本波零关
）；rustdoc 0 警告；ktest 453/0（U-2/U-3 首次运行时 16/16）。

验证证据链（完成态）：PASS_SLICING `pr3767_adaptation_wave_20260905` +
`overlay_022_upper_rejection_20260905` + `ktest_u2u3_execution_20260905`；
checker 收据与 run_evidence 在归档地
`components/pr3767-merge-20260904/`。

## 2. PR 进程先例与现状差距

`20260812-pr-draft-prep` 时代的计划：从 pure upstream main 切干净分支，
按 11-commit（1k~2k 行/commit）story-first 切分（C2–C10 惰性文件、C11
合龙）。**需刷新的差距**：① 该计划基于 8 月树，此后新增 PR #3767 pick +
适配波 + 022 修复（VFS 面与 overlayfs 面都演进了）；② 上游 main 已继续
漂移（本地 main 引用停在 864b9138b，需重新 fetch 评估）；③ legacy 删除
与新实现的故事切分要按当前 diff 重算。

## 3. Next-main-agent actions（待 user 指令逐步细化）

1. `git fetch upstream` 评估 main 漂移；决定 rebase 时点（当前分支基于
   `864b9138b`）。
2. PR 切分策略裁决：沿用 story-first 粒度还是按当前 diff 重切；确定
   VFS 配套修改（path/fs_apis/inode_handle 等）与 overlayfs 新实现的
   commit 归组。
3. PR 描述起草（含 PR #3767 关系声明：本分支 self-implements 其
   follow-up——rename new_dir dentry 化，及 022 修复等）。
4. 每个 PR commit 的验证义务沿用本轮 gate 链（make check / rustdoc /
   回归 / xfstests 全表）。

## 4. 工作区归档记录（本 handoff 之后的既成事实）

- 归档地：`/home/ayd/overlayfs-agents/`（= 原
  `kernel/core/src/fs/fs_impls/overlayfs/.agents/` 全量，含 protocol/
  priors/designdoc/refactor 归档/main-agent 全部 handoff/components
  证据/subagent-tasks packets）。
- 工作树中 `.agents` 移除并 commit（tracked 117 文件删除入库；历史中
  全部版本仍在 git）。
- skill 更新：`ovfs-main`/`ovfs-subagent`/`ovfs-checker` 的路径引用改指
  归档地。

## 5. Prohibitions

- PR 分支切割/rebase 时点/描述内容：未经 user 裁决不执行。
- 归档地 `/home/ayd/overlayfs-agents/` 为只读档案 + 续写地，不得删除或
  重构其历史结构。
