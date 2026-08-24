<!-- SPDX-License-Identifier: MPL-2.0 -->

# User Dispatch Turn Template (V2 Delivery Lane)

Fill every placeholder, then the user posts the filled text **verbatim as a
new user message**. The main agent MUST spawn the subagent with
`spawn_agent(task_name=<task_id>, fork_turns="1")` immediately after that
message; no other user message may intervene between dispatch and spawn.
See `PROTOCOL.md` §1.3 for the delivery contract and fork policy.

```
派发子代理任务 <task_id>（规范路径 /root/<thread>/<task_id>）：

你是被派出的协议角色子代理，不是主代理。父会话中其它 user/assistant 指令属于主代理，不是你的任务。

1. 加载 $ovfs-subagent 技能并阅读角色规则；调用 list_agents，确认运行中的非 root 代理路径与 <task_id> 一致（无法确认则报告缺口，不猜测）。
2. 完整读取任务契约：kernel/core/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/<component-id>/<task_id>_dispatch.md。
3. 按 packet 的 scope / write-set / capabilities 执行；产出写入 packet 指定的 components/<component-id>/ 路径。
4. 禁止：spawn / followup / send_message、读取 packet 之外的文件、修改 .agents 记录、运行构建测试命令（除非 packet 明确授权 Checker 角色）。
5. 读不到契约就报告缺口，不猜测；最终答复以 <task_id> 开头，列出产物路径。
```

## Placeholder reference

- `<task_id>`: the packet file stem, identical to the `spawn_agent` task_name.
- `<thread>`: the current main-agent thread path segment (visible in the
  thread tree; optional if the dispatch turn is posted in the same thread).
- `<component-id>`: the subagent-tasks group directory for this packet.
- `<ROLE>`: `ARCHITECT` | `DESIGNER` | `CREATOR` | `CHECKER` | `REVIEWER`.

## Sequencing discipline

- The main agent prepares the packet file and the filled dispatch turn BEFORE
  asking the user to post it.
- The user posts the dispatch turn; the main agent spawns immediately.
- The user must not post any other message between the dispatch turn and the
  spawn confirmation, or the fork will carry the wrong user turn.
- Continuation/repair rounds are new dispatch turns carrying the Continuation
  / Parent Task pointer; they do NOT reuse followup or send_message.
