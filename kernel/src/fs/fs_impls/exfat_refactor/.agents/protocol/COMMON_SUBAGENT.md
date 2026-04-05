<!-- SPDX-License-Identifier: MPL-2.0 -->

# Common Subagent Rules

This file is the shared baseline for ordinary delegated work on `exfat_refactor`.
Ordinary subagents should follow this file plus their role-specific packet rules; they should not need [`PROTOCOL.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md) for normal delegated work.

Every ordinary subagent must follow these rules:

1. Only perform the role and task named in the archived task packet.
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
12. Treat the task packet's prior inputs as the only authorized prior corpus for that step unless the packet explicitly allows a broader prior set.
13. When the task packet includes multiple prior layers, follow the packet's stated precedence. If the packet does not restate one, use the default precedence from the main protocol: Microsoft exFAT rules first, Linux exFAT implementation summary second, Asterinas architect priors as local context, and code-quality priors as engineering-quality constraints rather than semantic authority.
14. If the packet cites profile labels or section references such as `I-DESIGN` or `Q-CREATE`, do not silently widen yourself to the full prior files unless the packet explicitly allows that broader read scope.
15. If the assigned work appears to require missing prior material, stop and report the gap instead of silently substituting your own memory or unrelated documents.
16. Treat the packet's architectural-unit context as authoritative for the assigned step. Do not silently reinterpret an owner-internal slice as a standalone architectural boundary or public surface.
17. If the packet authorizes reads from `/home/halifuda/linux/fs/exfat/`, use those reads as exact Linux implementation context when needed. Do not assume the Linux summary alone is authoritative when the packet explicitly points you to source.
18. Do not treat the legacy Asterinas `exfat` implementation as the semantic target of the refactor. Use it only as local integration context unless the task packet explicitly records a required divergence or compatibility constraint.
19. If the packet authorizes a temporary staging surface, keep it explicitly temporary in both code comments and role artifacts by naming the future owner or removal condition. Do not hide staging work behind vague `TODO` markers.
20. Do not invent short helper wrappers or field-exposing accessors unless the packet or referenced artifact already states why another component needs that helper now.
21. Treat the packet's lane classification as part of scope control:
    - if the packet says the step is command-free, do not add compile, test, or runtime commands on your own;
    - if the packet says the step is serial with respect to another lane, do not proceed until that scheduling condition is satisfied.
22. If the packet says the step may overlap only with disjoint write-set lanes, treat any newly discovered overlap as a stop-and-report condition rather than editing through it.
23. If the packet says a checker execution stage must hold `.agents/locks/checker-execution.lock/`, do not run command-producing verification until that lock is acquired and `owner.toml` is written.
24. If the packet tells you to wait for the execution lock, wait quietly on the packet's retry interval instead of immediately escalating. Do not clear a stale lock unless the main agent explicitly authorized that review.

Subagent authority is strictly local.
Seeing the larger workflow does not authorize scheduler actions.
