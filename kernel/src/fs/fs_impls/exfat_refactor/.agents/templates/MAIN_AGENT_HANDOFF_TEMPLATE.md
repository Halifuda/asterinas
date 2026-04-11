<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff Template

Treat this file as the editable record of the current main-agent wave.
Update it during the wave, rewrite it when needed for clarity, and leave the next main agent a concise but complete state record.

## Metadata

- Fancy nickname:
- Date:
- Covered hours:
- Author:
- Workspace:
- Container or environment:
- Status:

## Environment Summary

- Image or base environment:
- Working path:
- Container name, if any:
- KVM status:
- Validated commands:
- Known environment blockers:

## Current Project State

- Current goal:
- Current phase:
- Active or next component:
- Latest accepted components:
- Components in progress:
- Blocked components:

## Active Work Slice Matrix

This is the scheduler-owned global view of currently adopted work slices.
Architect artifacts may recommend local candidate slices, but this matrix is the authoritative active plan.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-...` |  |  |  |  |  |  |  |  |  |

## Recent Decisions

List the decisions that materially shape future work.
Record both implementation-side and protocol-side decisions here.
If this wave changed scheduler-facing docs such as `PROTOCOL.md`, `README.md`, `TESTING_GUIDE.md`, `templates/`, or packet-shaping rules, summarize those changes here before ending the wave.

## Wave Record

- Scheduling or planning changes made in this wave:
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
- Protocol, template, or packet-shaping changes made in this wave:
- Important facts intentionally removed from earlier drafts because they are no longer relevant:

## Open Risks And Assumptions

List the assumptions that a future main agent must preserve, verify, or revisit.

## Recommended Next Actions

1. 
2. 
3. 

## Resume Checklist

- Use `$exfat-main-agent`.
- Read `README.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Read the active work-slice matrix in that handoff before dispatching or reshaping any lanes.
- Verify the environment summary above still matches reality.
- Confirm this handoff already reflects the material implementation and protocol changes from this wave before committing or handing off.
- Read `PROTOCOL.md` when protocol maintenance or an explicit scheduler-rule question is in scope.

## File Naming

Use a filename in the form `<fancy-nickname>-YYYYMMDD-HHMM-<summary>.md`.
