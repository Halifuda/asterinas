<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff: Prior Knowledge Layer & Micro-Feature Inventory Brainstorm

**Date / Time:** April 14, 2026, 16:50 CST
**Status:** Handed Over

## 1. Global State Pointer
*Always read `SYSTEM_BLUEPRINT.md` for the overarching project state. This section only notes immediate shifts made during this thread.*
- **Current Active Wave / Component:** Protocol Redesign Brainstorm (Prior Knowledge Layer Alignment)
- **Blueprint Updates Made:** None. We are intentionally holding all file modifications until the brainstorm phase concludes completely.

## 2. Thread Activity Log (The Active Wave)
*What did this specific main-agent session actually do? Keep it concise. Focus on scheduling and dispatching.*
- **Dispatches Sent:** None (In-depth alignment on the Strict Information Funnel with the user).
- **Acceptance Outcomes:** None.
- **Escalations / Deadlocks:** None.

## 3. Explicit Agent-Level Decisions
*Record non-automated choices made by the main agent during this session.*
- **Identified the Missing Prior**: The Architect's true starting point to define Macro-Owners is a **Micro-Feature Inventory**. The current prior layer is missing this critical baseline input.
- **Inventory Exhaustiveness**: We established that the upcoming Inventory must cover three dimensions:
  1. *Physical/On-Disk Layer*: e.g., Boot sector checksums, backup boot sectors, `NoFatChain` rules, Dentry state machines.
  2. *VFS/Interface Layer*: e.g., `O_APPEND`/`O_TRUNC` behaviors, atomic Rename/Rmdir guarantees, mount flags mappings.
  3. *BIO Substrate Layer*: e.g., Page/Sector boundaries, Block I/O Sleep/Block landscape. Without this, the Architect cannot build a deadlock-free Global Lock Topology.
- **Prior Cleansing Strategy**: The existing four prior files (`Microsoft-exFAT-spec.md`, `linux-exFAT-implementation-summary.md`, `ASTERINAS_ARCHITECT_PRIORS.md`, `ASTERINAS_CODE_QUALITY_PRIORS.md`) are contaminated with role labels and agent meta-instructions. They must be stripped back to pure, objective "fact dictionaries" (ores) with no instructional overlay.

## 4. Next Actions for the Next Thread (CRITICAL)
*When the next LLM context window starts, what is the EXACT first step it must take? Be highly prescriptive.*

1. **Research Undecided/Doubtful Parts in `linux-exFAT-implementation-summary.md`**:
   - Review any unresolved questions or technical details that require deeper verification in the current Linux exFAT implementation summary.

2. **Initiate the Writing of the Inventory (`ASTERINAS_MICRO_FEATURE_INVENTORY.md`)**:
   - Synthesize the requirements across the Physical Layer (MS Spec), Pipeline/VFS Layer (Linux), and Constraints/BIO Layer (Integration Priors) to write an extremely detailed micro-feature inventory, ensuring no "Owner Gaps" remain for the Architect phase.

3. **[COMPLETED TASKS] Prior Cleansing Tracking**:
   - **[COMPLETED] `linux-exFAT-implementation-summary.md`**: Fully populated the skeleton based on the Linux source code, clarifying VFS mappings, cache interactions, and global state transition pipelines.
   - **[COMPLETED] `Microsoft-exFAT-spec.md` (The Physical Truth Dictionary)**: The user has provided the complete official Microsoft technical specification document. We have named it to `Microsoft-exFAT-spec.md` and will use it directly as the definitive physical prior. No need to parse it into a skeleton. We also generated `Microsoft-exFAT-spec-index.md` listing all section headers and their line numbers. Future agents should consult this index first when looking for physical layouts to avoid scanning the entire 150KB file.
   - **[COMPLETED] Refactored `ASTERINAS_CODE_QUALITY_PRIORS.md`**: Stripped all role profiles and generic instructional meta-instructions. Now an objective Asterinas coding rulebook.
   - **[COMPLETED] Creation of `ASTERINAS_INTEGRATION_PRIORS.md`**: We successfully replaced the Architect priors with a rigid Dictionary for Designer constraints:
     - **Locks**: `SpinLock` preemption limits vs `Mutex` sleep safety.
     - **BIO**: `BlockDevice` extension methods (`read_blocks`, `write_blocks_async`) + `BioWaiter` sleep bounds.
     - **PageCache**: Fragmentation mapping and `PageCacheBackend`.
     - **VFS/Inode**: Exhaustive `FileSystem` hooks and `Inode` flags coverage (`O_APPEND`, `FallocMode`).
     - **Errno**: Precise mappings without flawed assumptions (explicitly blocking `ENOSYS`, preferring `EOPNOTSUPP` and `EROFS`).

5. **Exit Brainstorm Phase & Update Blueprint**:
   - Once all Priors are purified and the Inventory instantiated, update `SYSTEM_BLUEPRINT.md` and transition into the execution phase (Architecture -> Design -> Creation).
