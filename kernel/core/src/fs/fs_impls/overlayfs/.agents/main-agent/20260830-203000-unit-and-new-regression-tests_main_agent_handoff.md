<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: 2026-08-30 单测与新回归测试规划（开题）

**Date / Time:** 2026-08-30 20:30 CST
**Status:** `SPEC-DESIGN IN FLIGHT — 2026-08-31：user 圈定全部七测（U-1/U-2/U-3 + R-1~R-4）一次设计；PROTOCOL rule 17 已正式修订（ktest 单测限权放行）；Designer packet `task_designer_test_assets_20260831` 已派发（Direct Spawn Lane）。产品实现仍未动工。`
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

（2026-08-31 起被 §6 部分解除，见下。）

不派发任何测试相关 packet；不改 `test/` 任何文件；不改生产代码；
不运行任何测试（xfstests/regression/ktest 均未授权）。

## 6. 2026-08-31 开工记录：裁决落地与只读提案

**User 裁决（本日）：**
- **Open question ① 裁定为放宽**：允许一定的 ktest 作为 overlayfs 单测
  载体（选项 (b)）。依据上游 PR#3708 tatetian 意见（2026-08-15 "On
  testing" 节）：单测**尽量少写**，只针对自限性强但过程/接口复杂的部分
  （其原例即 `OverlayMountOptions::parse`），且写单测的过程用于暴露
  under-specified 行为、反哺 spec。PROTOCOL rule 17 的正式修订随单测
  pass 派发时落文（记录本条为决策依据）。
- **Open question ②/③**：回归线新增「NOTRUN 弥补」目标维度（见下提案），
  用例圈定与写集边界待 user 对本提案拍板。

**只读调研（本日完成）：**
- PR#3708 评审意见已提取（gh api）；wave7 统一账（§8/§8.1/§8.2）已复核；
  上游 overlay/004/008/015/025/027/028/041/066 源码已读，意图归类完成。
- 本地能力核实：flock 已实现（`syscall/flock.rs`）；VFS 无 SGID 继承实现；
  POSIX ACL/userns/fileattr ioctl 均缺失；回归 C 测试可用 fork+setuid
  数字 uid 规避 fsgqa 环境门（无需 passwd 条目）。

**测试提案（待 user 圈定后走 Designer 切片）：**

单测线（ktest，`#[ktest]`，参照 `kernel/libs/xarray/src/test.rs` 形态）：
- U-1 `OverlayMountOptions::parse`（`fs/mount/options.rs:62`）——tatetian
  点名目标；需先在 doc/spec 冻结：引号值、转义、重复 key、空值、
  uuid/xino 非法值等口径。
- U-2 xattr 名映射纯函数组（`used_full_name`/`present_xattr_names`/
  classify，`inode/xattr.rs`）——转义/剥段/隐藏语义，纯函数零依赖。
- U-3 xino 矩阵（`inode/identity.rs`，`XinoMode` 定义于 `fs/policy.rs:36`）
  ——fsid 高位拼装/回读 + 溢出显式 fallback；fallback 口径目前
  under-specified，写测即冻 spec（2026-08-31 复核后建议纳入，lean 若干
  encode/decode/fallback 例）。`RealObjectKey` 层序经复核为平凡比较器，
  不单独成测，从候选剔除。

回归线（`test/initramfs/src/regression/fs/overlayfs/`，自包含风格）：
- R-1 copy-up 属主与权限语义（弥补 004+008，fsgqa 门规避）：chmod 触发
  copy-up 且 upper 收到新 mode/lower 不动；无权 uid（fork+setuid）被拒；
  create-over-whiteout 属主 = 创建者 fsuid/fsgid 而非 mounter。
- R-2 sparse copy-up 内容一致性（弥补 066，去 xfs_io/fiemap 依赖）：
  truncate 空洞文件 copy-up 后读回全零、size 保持。
- R-3 flock over copy-up（028，原未测候选）：lower 文件持锁 → 写打开
  copy-up → 锁跨 copy-up 仍生效。**2026-08-31 代码核实：overlayfs 对
  flock 零处理**（无任何 overlay 侧 flock 代码；`FlockItem` 锁挂在真实
  文件 `owner: Weak<dyn FileLike>` 上，copy-up 换底后新写路径落在另一个
  真实 inode，锁大概率失效）。因此 R-3 定位为**characterization test**：
  预期 FAIL，把"行为未知"固化为可复现的缺口证据，修复后转为回归门。
- R-4 xino=on 下 d_ino 恒等（041 意图；与 §2 候选 ④ 合并扩展，
  non-samefs 变体）。
- §2 原候选 ①转义语义 ②userxattr ③嵌套叠加 维持待圈定，与本提案
  并行不冲突。

**明确不可弥补（保持 NOTRUN，缺口文档化）：** 020（userns）、
023/025（POSIX ACL）、027/030/035/040/075/076/078（fileattr ioctl）、
021（aio）、066 的 fiemap 半边、041 的 show_options 回显门本身。
015 部分可弥补（SGID 继承依赖 base VFS 缺口，若做需先登记该缺口）。

**Next actions：** user 圈定 R-*/U-* 清单 → Designer bounded 冻结面
（含 PROTOCOL rule 17 修订文）→ per-pass Creator/Checker。本轮仍不改
`test/` 任何文件、不跑任何测试。

**2026-08-31 补记（适用范围澄清）：** book guideline
`add-regression-tests` 仅强制 **bug fix 随带测试**；feature-add PR 无
测试强制（for-development README 的 "proven by tests" 是评审问句而非
硬规则）。因此本线 R-*/U-* 是**项目决策**（导师 PR#3708 意见 + NOTRUN
弥补目标），不是 book 合规义务；book 规则唯一咬合点是任何修复（如 R-3
的 flock 缺口）落地时必须随带能抓到它的测试并引用来源。回归测试写作
本身受 testing.md 其余三则（test-visible-behavior / use-assertions /
test-cleanup）约束。

## 7. 2026-08-31 派发记录：七测一次设计

- **User 裁决补充**：七测全选（U-1/U-2/U-3 + R-1~R-4），一个 Designer
  一次设计完；上下文归因统一改用「上游 PR#3708 评审意见 + user 决策」
  表述（用户要求移除会话内人称化指称）。
- **协议修订（本日落文，main agent 维护）**：PROTOCOL rule 17 重写为限权
  放行（ktest 单测仅限纯逻辑面、Designer artifact 点名、packet 授权、
  Checker 经 `make ktest` Validation Run 执行）；同步修订 Core Terms
  Checker Pass、rule 4、rule 9、§1.3(f)、§2 Designer、§3 gate 2、
  `protocol/DESIGNER.md` 两处 ktest 条款。
- **派发**：packet
  `subagent-tasks/test-assets-20260831/task_designer_test_assets_20260831_dispatch.md`
  （Meso Spec / design / Normal；write-set = components/test-assets-20260831/
  下 spec+validation 两文件；Direct Spawn Lane，自包含首消息含 §1.3 (a)-(g)）。
- **Next main-agent actions**：收集 Designer 报告 → 结构验收（§2 必备
  结构齐备、七 micro 全覆盖、write-set 零越界、U-* 冻结口径表完整）→
  通过后由主代理做 PASS_SLICING 决策（Creator pass 边界 + R-3
  characterization/落地分离）→ 修订 PROTOCOL 时一并登记本 handoff。
- **结构验收（本日，ACCEPTED）**：两产物齐备 packet §2 全部必备结构
  （spec 640 行：§1 适配 Rely-Guarantee + §2 七 micro 追溯表 + §3 U-* 含
  8+8+8 测函数清单与冻结口径表 + §4 R-* 含编排/断言/cleanup/注册行 +
  §5 复杂度与命名自查；validation 145 行：映射表 + Checker 义务 + 集成
  观察含显式留白）。Write-Set 零越界（git status 仅 main agent 自有
  三文件改动 + 两新产物）；无 .rs/.c 产出；无 pass slicing。事实纠偏
  一处：目标符号实为 `MountOptions::parse`（迁移后命名，非
  `OverlayMountOptions`），Designer 已按实际符号核线设计。R-4 的双
  独立 tmpfs fixture 经主代理复核可行（`fs_impls/tmpfs/fs.rs` 存在，
  `mount/listmount.c` 已有双 tmpfs 挂载先例）。
- **遗留（下一 session）**：PASS_SLICING 决策未做——候选切法：
  Creator pass A = U 线三 ktest 模块（含被测文件内 doc 冻结表落文），
  pass B = R-1/R-2/R-4 三回归（test/ 写集），R-3 独立（characterization
  run 先行、与 flock 修复同 pass 落地）。test/ 写集边界（此前 open
  question ③）随 pass B packet 圈定。

## 8. 2026-08-31 派发记录：MountOptions v2 设计（user 定纲）

- **User 定纲**：派 Designer 全面设计，涉及 options.rs/policy.rs/mod.rs/
  capabilities.rs 至少四文件；原则：runtime 所用状态归 policy、options 收
  窄可见性（现有违例一并改）；options 解析全部已知 option；实现三类降级
  （未实现自动降级/冲突降级/capability 降级）；warn vs fail 参考 Linux；
  fringe 六项由 Designer 裁决（override_creds 专项考虑）。
- **派发与验收**：packet
  `subagent-tasks/mount-options-v2-20260831/task_designer_mount_options_v2_20260831_dispatch.md`
  （Meso Spec / design / High；父 Meso `mount_resource_policy`）→
  产物 782+168 行，结构齐备 → 验收发现 validation §3.2 四组 name↔id
  映射错 + "runnable expected-FAIL" 结论不成立 → **continuation 1**
  （packet `..._continuation_1_dispatch.md`，附主代理实测上游 gate 证据）
  修复完成：Designer 独立 curl 复核 28 用例零出入，spec 经全量 grep 确认
  无同源错误、零修改。**ACCEPTED（post continuation 1）**。
- **六 MO 决议摘要**：MO1 17 key → 8 full（含新 full 项 lowerdir+）、6
  parse-only-degrade（redirect_dir/index/nfs_export/metacopy/verity/fsync
  [volatile 别名]）、2 reject（datadir+；override_creds 正形式，其否定
  形式 nooverride_creds 接受为 silent no-op）。MO2 降级载体裁决：不需要
  `degraded_to` 载体（零读者/易陈旧），options 存原始意图（私有）、生效
  态 mount 期计算、原因一次性日志披露；capability 降级沿用 uuid 三态范
  式，零新 probe。MO3 逐位点 Linux parity 表 + warn/info 文案草案；两处
  parity 补齐（uuid=on 只读挂载无效、xino=on 同 fs）。MO4 消费面审计：
  零运行期 MountOptions 读者（不随 OverlayFs 存储）；违例为
  `assemble(&MountOptions)` 跨模块回读、struct/3 字段可见性过宽、
  xattr_prefix/xino 默认值双推导；capabilities.rs 零签名变化。MO5
  override_creds 专项：上游 v6.15 起 flag_no 双字面形式，正形式记录
  option-issuing task creds 且无 userns 语境时上游自身 EINVAL
  （params.c:705-708）→ 本地无 credentials API（policy.rs TODO 即缺口），
  硬拒为 parity 忠实选择；TODO 是翻转为 full 的既录触发器。MO6 U-1
  delta：改 2 行、增约 24 行冻结 case、8 个新 `#[ktest]` 签名，可机械
  套用。
- **重要台账修正（纠正主代理此前对 user 的表述）**：parse 接受
  降级**不**使 index/redirect_dir/metacopy/nfs_export 门控组（27 例）
  变为可运行——`_check_overlay_feature` 三级链（module 参数 → show_options
  回显 → touch）先于一切断言，用例在 level 1/2 即 NOTRUN（module-param
  面与 #3706 show_options 缺口均在本设计之外）。台账语义为两阶段：
  Stage 1（本设计）mount-EINVAL → 挂载成功+degrade warn、用例仍 NOTRUN；
  Stage 2（需 user 单独授权的 VFS 缺口工作）后才变 runnable expected-FAIL。
  另：Designer 发现 wave7 旧账"mount-EINVAL NOTRUN"归因最多是 gate 挂载
  步的表象（level-1 检查先于任何挂载）；wave7 台账本身未改动。
- **网络事实**：子代理外网曾间歇阻断，改用本地 `/home/ayd/linux`
  （v7.2.0-rc3）树完成权威精读，全部位点带 params.c/super.c 行号，无
  inferred-unverified 条目。
- **Next main-agent actions**：PASS_SLICING 决策（候选：Creator pass V2 =
  MountOptions v2 生产落地 [MO1-5]；pass A 修订 = test-assets U-1 按 MO6
  delta 有界修订后与 V2 pass 同 wave 实现；R 线 pass B 维持待写集边界）。
  全部 Creator 派发待 user 指令。

## 9. 2026-08-31 执行记录：pass_v2_mount_options 实现闭环（生产代码）

- **Protocol 命令修正**：CREATOR.md:35 / CHECKER.md:43 陈旧编译命令
  `cargo check -p aster-kernel`（该包已不存在）→
  `cargo osdk check -p aster-core`（包名经 kernel/core/Cargo.toml 核实，
  osdk Check 子命令经 osdk/src/cli.rs:62,98-99 核实；容器
  codex-asterinas-dev 运行中）。
- **Creator**（task_creator_pass_v2_mount_options_20260831，
  compile_preflight 授权）：
  - run_1：MO1-5 生产面全实现（17 key 矩阵、verify 冲突+降级、A1 info!、
    MO4 归属重构含 assemble 6 参签名与可见性收窄、MO5 全裁决），编译
    exit 0；census Full（3 enum + 6 私有字段 + 1 owner-private verify +
    收窄，无 free helper；degraded_to 载体按 spec 未引入）。
  - Deviation D1（packet 写集缺陷：spec MO3-R1 位点在 inode/identity.rs
    而原写集四文件）→ Continuation 1（同 Creator 线程，SendMessage）：写
    集扩展第五文件、R1 补齐（逐字冻结文案）、run_2 编译 exit 0、census
    无新实体、D1 关闭。
  - D2-D6 六项偏差全记录（均行为等价/记账类，见 Creator 报告）。
- **Reviewer**（task_reviewer_pass_v2_mount_options_20260831）：**PASS**
  一轮通过，无需复工。零 structural/semantic finding；D1-D6 逐条
  disposition 接受；census 独立复核一致；边界检查通过（零 ktest、写集零
  越界、既有 7 key 零回退、零新锁）；无 line-level edits。两条 nit 仅供
  记录不阻塞：mod.rs:60-62 A1 注释措辞（"identity flow" 可更精确）、
  options.rs:225-240 lowerdir+ arm 检查顺序不对称（多违规输入仅文案
  差异）。
- **Write-Set 终态**：fs/mount/options.rs、fs/policy.rs、fs/mount/mod.rs、
  inode/identity.rs（capabilities.rs 零 diff，符合 spec 预期）。改动未
  提交（per-pass commit 待 user 指令）。
- **修复循环机制已验证**：同 Creator 复工经 SendMessage 直达
  （agent_1392fcfc-143f-47df-a101-51ded0119bed）、同 Reviewer 复验通道
  （agent_14afa231-6308-464e-9593-8d817c8a0da5）。
- **Next main-agent actions**：(1) test-assets U-1 有界修订 pass（按
  spec §5 MO6 delta，ktest 授权经 amended rule 17）；(2) R 线 pass B
  （写集边界待 user 圈定）；(3) 本 pass 的 per-pass commit 与（可选）
  make check 全量 parity，待 user 指令。

## 10. 2026-08-31 执行记录：ktest 单测 pass（U-1/U-2/U-3，板正纪律）

- **前置**：pass V2 已 commit（`287b1ea5d`，生产 4 文件 + 6 .agents 记录；
  components/ 与 subagent-tasks/ 由 .gitignore 刻意排除不入库）。
- **Creator**（task_creator_pass_unit_tests_20260831，ktest 授权 per
  amended rule 17）：
  - 24 个 `#[ktest]`（U-1 16 = 8 基线 + 8 delta；U-2 8；U-3 8）+ 11 个
    冻结签名 helper，落 `fs/mount/options/test.rs`、`inode/xattr/test.rs`、
    `inode/identity/test.rs` 三新文件；三父文件各 +3 行
    `#[cfg(ktest)] mod test;` 声明，零语义改动。生产实体 = 0（census 单列
    test-only）。
  - **板正纪律落实**：断言期望唯一来源 = 两 spec 冻结表；被测源码仅读
    接口信息；**零测试执行**（两条编译 gate：常规 + `--cfg ktests`，
    run_1/2/3 均记录；--ktests 首跑一次 exit 101 为 Creator 笔误、
    Write-Set 内修复后通过）。
  - Spec 歧义备案 5 项（uuid/xino 输入行并接 base lowerdir、行→函数
    按名分组、缩写格展开、U-3 未钉输入细节、import 行）——均字面读法
    实现并记录，未借生产代码消歧。
- **主代理裁决（continuation 1，同 Creator 线程）**：
  - Suspected mismatch #1（test-assets spec 冻结行 200 "empty entries
    skipped" 输入列与期望列自相矛盾）判定为 Designer 输入列笔误：spec
    输入列已修订为含 `default_permissions`（期望列不变）；Creator 同步
    测试串 + rustfmt 三行折行。
  - 歧义 #5（冻结 `use crate::prelude::*;` 3 处 unused import，会挂
    clippy -Dwarnings 门）：主代理修订 spec 三处签名块删除该行；
    Creator 删除三测试文件对应 import。
  - run_3（`cargo osdk check --ktests -p aster-core`）exit 0、零 warning。
- **状态**：本 pass 工作树改动未提交（6 文件，待 user 指令）；测试执行
  验证（`make ktest` Checker Validation Run）未做——冻结测试与生产的
  真实对齐由该 run 揭示。
- **Next main-agent actions**：(1) `make ktest` Checker Validation Run
  （待 user 授权执行）；(2) 本 pass per-pass commit（待 user 指令）；
  (3) R 线 pass B（test/ 写集边界待 user 圈定）；(4) Reviewer 静态门
  （可并入后续 wave 收尾或单独派发）。

## 11. 2026-08-31 追记：单测布局改回单一文件（continuation 2）

- **User 裁决**：拒绝 `options.rs` 旁的 `options/test.rs` 子目录形态
  （Rust 2018 子模块路径规则的产物，单测 pass 刚引入）。要求单文件布局。
- **裁决依据**：测试必须为目标模块子模块才能访问 module-private 项
  （U-2 三个目标函数均私有），因此唯一单文件形态 = 内联
  `#[cfg(ktest)] mod test { … }` 块追加于父文件末尾。
- **执行**：test-assets spec 布局条款三处 + 路径规则注记由主代理修订
  （内联形态 + rationale）；同 Creator 线程转换（内容逐字承继，脚本
  反缩进比对验证零语义差异），三目录删除；run_4 双 gate exit 0 零
  warning。
- **Write-set 终态**：三父文件（options.rs 1086 行 / xattr.rs 956 行 /
  identity.rs 995 行，含内联测试块）；无独立 test.rs。
- **状态**：工作树未提交；`make ktest` Checker Validation Run 与 Reviewer
  静态门、per-pass commit 均待 user 指令。

## 12. 2026-08-31 执行记录：ktest 验证 run（含一次失败归因与修复）

- **前置 commit**：单测 pass 已 commit（`0ab6442f6`，3 源文件 + 2 记账）。
- **ktest 命令形态（查 CI 后确定并实证）**：CI ktest = `make ktest
  NETDEV=tap`（根 cwd，遍历全部 30 个 default-member crate，每 crate 一次
  QEMU 启动）。**crate 级 scoping**：`cargo metadata` 的
  `workspace_default_members` 随 cwd 收缩（根 = 30，`kernel/core` = 1 =
  aster-core）→ `cd kernel/core && cargo osdk test` = 单次 QEMU 启动仅跑
  aster-core。OSDK.toml 经 workspace_root 回退解析、`$(./tools/…)` 以
  manifest 目录为 workdir，cwd 不影响（manifest.rs:138-140 实证）。
- **Checker run_1（task_checker_unit_tests_ktest_20260831）**：24 测
  23 PASS / 1 FAIL。空探针实证退出码通道可靠（0=Success/1=Failed）。
- **环境缺口 (a) 静默控制台（重要）**：aster-core 的 early cmdline parser
  （父提交 `61e1ad700` 组件搬迁带入的强符号）要求 cmdline 含 `earlycon`，
  而 OSDK 测试内核 cmdline 为空 → ktest 全部输出被丢弃（串口日志仅 403
  字节引导桩）。**无需改仓库即可恢复**：`cargo osdk test
  --kcmd-args="earlycon"`（403→4322 字节实证）。此缺陷影响全部 aster-core
  ktest 的可观测性（含 CI），建议后续登记 VFS/OSDK 缺口（修复归属需
  user 裁定：cmdline crate 或 OSDK 默认参数）。环境 (b)：WSL2 容器无
  KVM，QEMU 走 TCG，不影响结论。
- **失败归因（earlycon 诊断 run_27 取得断言原文）**：
  `lower_id_wire_roundtrip_preserves_identity` 挂在 encodable 期望断言
  （actual passthrough `{dev(1,1), 0x1234}` vs expected
  `{dev(9,9), (3<<48)|0x1234}`），且 build_policy 时打出了 R1 INFO——
  单层 lower 策略 `is_all_layers_same_fs` 空洞为真 → same-fs passthrough。
  **裁决：spec 行 428 自相矛盾（单层策略 + encode 行期望），生产无 bug**
  （passthrough 与 Linux numfs==1 语义及 spec 自己的 same-fs 行一致）。
  test-assets spec 行 428 已由主代理修订为 encode 行的双层表
  `[(3,dev(1,1),100),(4,dev(2,2),200)]`；同 Creator continuation 3 同步
  测试（期望字面值与其余断言全部未动）。
- **最终验证**：单测过滤 run exit 0；全量 `cargo osdk test
  --kcmd-args="earlycon"`（cwd=kernel/core）→ `test result: ok.
  138 passed; 0 failed`——**aster-core 138 测全绿**（含本 pass 24 + 既有
  114，后者即我们生产改动的回归信号）。
- **状态**：工作树未提交：identity.rs（continuation 3 一处层表修正）+
  handoff/PASS_SLICING 记账（spec 在 gitignore 的 components/ 内）。
- **Next main-agent actions**：(1) 修复后增量 commit（待 user 指令）；
  (2) R 线 pass B（test/ 写集待圈定）；(3) ktest 静默控制台缺口登记
  （待 user 裁定归属）；(4) Reviewer 静态门（可并入 wave 收尾）。
