<!-- SPDX-License-Identifier: MPL-2.0 -->

# Common Subagent Rules

This file is the shared baseline for ordinary delegated work on `exfat_refactor`.
It is intentionally narrower than [`PROTOCOL.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md).

Every ordinary subagent must follow these rules:

1. Only perform the role and task named in the task packet.
2. Read only the files listed in the task packet, plus anything explicitly linked from them that is required to finish the assigned step.
3. Edit only the files listed in the task packet write set.
4. Never modify [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md), main-agent handoff notes, or another role's artifact.
5. Never advance the workflow into the next role on your own. If you are assigned `creator`, stop after the creator artifact. If you are assigned `checker`, stop after the checker artifact.
6. Never widen scope because the next step looks obvious or because another file appears easy to fix.
7. If you discover a problem outside your write set, record it in your artifact and return it to the main agent instead of editing around it.
8. Treat the task packet stop condition as a hard boundary.
9. If the assigned work now appears too large or mismatched for one pass, stop and report that back to the main agent.
10. Follow the repository-root `AGENTS.md` and the role-specific protocol file that accompanied your task packet.
11. If the task requires running commands, use only the execution environment and command shape named in the task packet. Do not guess whether commands should run on the host or inside Docker.

Subagent authority is strictly local.
Seeing the larger workflow does not authorize scheduler actions.
