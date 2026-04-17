<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: [Thread Name / Session Nickname]

**Date / Time:** [e.g., April 14, 2026, 16:00 CST]
**Status:** [Active | Closed / Handed Over]

## 1. Global State Pointer
*Always read `SYSTEM_BLUEPRINT.md` for the overarching project state. This section only notes immediate shifts made during this thread.*
- **Current Active Wave / Pass:** [e.g., EXR-WRITEAT-P02]
- **Blueprint Updates Made:** [Yes/No - Summarize briefly, e.g., "Moved EXR-ALLOC from Designer to Creator loop"]

## 2. Pass Slicing Decisions
*Record the non-default pass boundaries chosen by the main agent. This is mandatory whenever a meso-component is split into multiple Creator/Checker passes.*
- `[Pass ID]` under `[Parent Meso-Component]` covers `[Micro A, Micro B, Micro C]`

## 3. Thread Activity Log (The Active Wave)
*What did this specific main-agent session actually do? Keep it concise. Focus on scheduling and dispatching.*
- **Dispatches Sent:** 
  - `[Subagent ID]` dispatched for `[Parent Meso] / [Pass ID or meso-wide task]` using `[Packet Path]`
- **Acceptance Outcomes:**
  - `[Parent Meso] - [Role] - [Pass ID or meso-wide task]` -> [Accepted (Template Validated) | Rejected (Reason)]
- **Escalations / Deadlocks:**
  - [Note if any Checker->Creator loop hit the 5-retry limit and was escalated here.]

## 4. Explicit Agent-Level Decisions
*Record non-automated choices made by the main agent during this session. (e.g., clearing stale locks, skipping final reviewer, reopening a Creator Pass after an integration failure).*
- [List decisions here, or "None"]

## 5. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*
1. [e.g., "Wait for the Subagent currently running EXR-ALLOC Creator to finish, then review its template."]
2. [e.g., "Dispatch the synchronized Checker Pass for EXR-WRITEAT-P02 using the Creator receipt and the Designer ktest."]
