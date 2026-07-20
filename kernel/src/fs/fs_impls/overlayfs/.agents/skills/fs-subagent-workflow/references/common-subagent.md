# Common Subagent Rules

Use this note together with the archived packet and the role-specific note.

## Scope rules

- Perform only the role and task named in the packet.
- Read only the files the packet authorizes, plus exact linked repo files needed by those artifacts.
- Edit only the files in the packet write-set.
- Do not modify `SYSTEM_BLUEPRINT.md`, main-agent handoff notes, or another role's artifact unless the packet explicitly authorizes it.
- Do not advance the workflow into the next role on your own.

## Packet authority

- Treat the packet stop condition as a hard boundary.
- Treat the packet's input file list as the authorized context window for the step.
- If required material is missing, stop and report the gap rather than substituting memory.
- Do not inflate a dispatch stub into an architectural tutorial.

## Command discipline

- If the packet is command-free, do not add compile, test, or runtime commands.
- Checker owns the execution lane by default.
- Creator, Designer, Reviewer, and Architect stay command-free unless the packet explicitly grants a narrow exception.
- If a packet authorizes commands, use only the named environment and command shape.
- If checker lock is required, do not run command-producing work until the lock is acquired.

## Code navigation

- When scoped Rust code inspection is allowed by the packet, prefer the `ra-code-nav` skill (LSIF index + `jq`) before broad `rg` / file search.
- Use the skill's recipes: workspace symbol search by moniker identifier, goto-definition by moniker, find-references by moniker, hover by moniker, and document symbols by file URI. See `.agents/skills/ra-code-nav/SKILL.md` for copy-paste recipes.
- This is read-only LSP semantic navigation. It does not expand the packet's authorized context and does not replace role-specific reasoning.
