<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: [Thread Name / Session Nickname]

**Date / Time:** [e.g., April 14, 2026, 16:00 CST]
**Status:** [Active | Closed / Handed Over]

## 1. Global State Pointer
*Always read `SYSTEM_BLUEPRINT.md` for the overarching project state. This section only notes immediate shifts made during this thread.*
- **Current Active Wave / Component:** [e.g., EXR-ALLOC-27]
- **Blueprint Updates Made:** [Yes/No - Summarize briefly, e.g., "Moved EXR-ALLOC from Designer to Creator loop"]

## 2. Thread Activity Log (The Active Wave)
*What did this specific main-agent session actually do? Keep it concise. Focus on scheduling and dispatching.*
- **Dispatches Sent:** 
  - `[Subagent ID]` dispatched for `[Component]` using `[Packet Path]`
- **Acceptance Outcomes:**
  - `[Component] - [Role]` -> [Accepted (Template Validated) | Rejected (Reason)]
- **Escalations / Deadlocks:**
  - [Note if any Checker->Creator loop hit the 5-retry limit and was escalated here.]

## 3. Explicit Agent-Level Decisions
*Record non-automated choices made by the main agent during this session. (e.g., clearing stale locks, skipping final reviewer, altering architect's recommended slice).*
- [List decisions here, or "None"]

## 4. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*
1. [e.g., "Wait for the Subagent currently running EXR-ALLOC Creator to finish, then review its template."]
2. [e.g., "Dispatch Checker for EXR-ALLOC using the Designer's test spec."]
