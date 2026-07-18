# Dispatch And Funnel

Use this note when authoring or reviewing a packet.

## Packet goal

A dispatch stub should route the assignee to the exact repo files and the exact output template, while leaving architectural reasoning in the upstream artifacts instead of in the packet text.

## Required packet shape

- archive path under `.agents/subagent-tasks/<component-id>/`
- role id
- component or task group
- parent meso-component when applicable
- covered micro-features when applicable
- exact read-only input file paths
- exact output template path
- exact output destination path
- minimal execution-specific overrides only

## Information funnel rules

- Architect may receive heavy priors.
- Designer receives Architect outputs and local component context.
- Creator receives the Designer contract plus code-quality priors.
- Checker receives Designer validation obligations plus Creator receipts.
- Do not leak Microsoft spec summaries or Linux implementation digests into Creator packets.

## Template rule

Use `.agents/protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md` as the packet frame.
The packet is not a tutorial and not a design memo.
