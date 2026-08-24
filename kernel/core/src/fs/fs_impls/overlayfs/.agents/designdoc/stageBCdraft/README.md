<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlay FS 基础设计：Stage B/C 最终稿

**状态：** Stage B/C 已完成；进入 Stage D 前的最终设计记录
**范围：** Ownership/Lifecycle 与 Invariants/Concurrency 的联合设计
**前置：** [Stage A 草稿](../stageAdraft.md)
**定位：** 设计模块划分和交互顺序，不冻结 Rust 类型、函数签名或实现 pass

## 文件结构

每个 B/C 阶段使用独立文件。阅读或继续设计某个阶段时，只需加载对应文件；
本索引保留模块顺序、状态和共同推进规则。

| 阶段 | 文件 | 状态 |
| --- | --- | --- |
| B/C-1 | [Mount、Layer Stack、Upper/Workdir](BC-1-mount-layer-upper-workdir.md) | 已确认 |
| B/C-2 | [Projection、Identity、Lookup](BC-2-projection-identity-lookup.md) | 已完成 |
| B/C-3 | [Merged Directory、Readdir](BC-3-merged-directory-readdir.md) | 已完成 |
| B/C-4 | [Copy-up、File I/O、Page Cache](BC-4-copy-up-file-io-page-cache.md) | 已完成 |
| B/C-5 | [Metadata、Permission、Xattr](BC-5-metadata-permission-xattr.md) | 已完成 |
| B/C-6 | [Directory Mutation、Whiteout](BC-6-directory-mutation-whiteout.md) | 已完成 |
| B/C-7 | [Advanced Identity、Export、Data Features](BC-7-advanced-identity-export-data.md) | 已完成 |
| B/C-8 | [Cross-Module Reconciliation](BC-8-cross-module-reconciliation.md) | 已完成 |

## 设计方法

Stage B 和 Stage C 不按“先完成全部 ownership，再完成全部 concurrency”的方式
串行推进。每个语义模块同时回答两类问题：

1. 哪个 Overlay 责任主体拥有状态、负责状态生命周期和持久化协调；
2. 该状态在并发访问、阻塞底层调用、回调重入、失败和 teardown 中必须满足
   哪些不变量。

每个模块完成时，应形成一组可审阅的设计结果：

- ownership 和 lifecycle 模型；
- 状态发布、更新、失效和销毁条件；
- 模块内部的不变量和并发边界；
- 阻塞/BIO 和可能重入的 underlying VFS 调用边界；
- 与已完成模块的依赖和交接约束；
- 仍未解决的问题和需要用户确认的决策。

每个模块都是一次交互式设计 checkpoint。八个模块均已完成确认；模块完成不
代表对应生产代码已经实现。

## 推进规则

模块按 B/C-1 到 B/C-8 的顺序推进，但每个模块内部都同时讨论 ownership、
lifecycle、invariants 和 concurrency。每轮先由主 agent 提出一个有限主题，再
由用户确认、修正或延后；确认结果写回对应阶段文件和 live handoff。

本草稿不定义 Creator pass，不改变 `SYSTEM_BLUEPRINT.md` 或 `PASS_SLICING.md`，
也不授权任何生产代码、测试或运行时验证工作。
