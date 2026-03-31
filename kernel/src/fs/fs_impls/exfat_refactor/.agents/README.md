<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Workspace

This directory stores the operating protocol for the parallel exFAT refactor module.

The project has two goals:

1. Refactor the exFAT implementation into clearer, better-specified components.
2. Explore the practical automation boundary of LLM agents when building filesystem code without losing engineering control.

- [`PROJECT_BRIEF.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROJECT_BRIEF.md) records the original project framing, role intent, and why the workflow is structured this way.
- [`ASTERINAS_ARCHITECT_PRIORS.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md) records the Asterinas-local knowledge an architect should treat as prior context before splitting components.
- [`TESTING_GUIDE.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md) records how exFAT ktests should be written, selected, and executed in the validated container workflow.
- [`PROTOCOL.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md) is the normative workflow.
- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) is the scheduler-owned task board.
- `templates/` contains the required handoff formats for each agent role.
- Component artifacts use chronological two-digit prefixes such as `00`, `01`, `02`, then `10`, `11`, `12`, so filesystem ordering matches pass ordering.
- `templates/MAIN_AGENT_HANDOFF_TEMPLATE.md` is the checkpoint handoff format for cross-thread or cross-machine continuity.

All agents working on `exfat_refactor` must follow both the repository-level `AGENTS.md` and this directory-local protocol.
