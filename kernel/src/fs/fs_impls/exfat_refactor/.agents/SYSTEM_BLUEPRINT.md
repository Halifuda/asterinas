<!-- SPDX-License-Identifier: MPL-2.0 -->

# System Blueprint & Dispatch Ledger

This file is the dynamic central blackboard and tracker for the multi-agent exFAT refactor. It tracks the progress of the Top-Down Strict Protocol, ensuring all artifacts are generated in the correct sequence and no concurrency invariants (locks/owner gaps) are violated. Managers and Agents must continuously update this ledger as work progresses.

## 1. Macro Topology & Global Status
<!-- Tracks the foundational Phase 1 architecture. This must be completed and frozen before downstream Meso-Components are processed. -->

- [ ] **Phase 1: Global Backbone** (`macro_00_global_topology.md`)
  - **Status**: Pending

## 2. Meso-Component Pipeline Index
<!-- Tracks the high-level end-to-end lifecycle of each Meso-Component. 
This tracks the macro-to-meso transition and architectural/design sign-off for the components as a whole.
Creator/Checker slicing happens later and is decided by the main agent. -->

| Meso-Component | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| *(Pending)* | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Pending |

## 3. Pass Tracking & Dispatch (Information Funnel)
<!-- Granular tracking of Creator/Checker/Reviewer passes linked to their parent Meso-Components.
The main agent decides which Micro-Features travel together in each pass. This queue must show the parent meso scope and the covered-micro set explicitly. -->

| Pass ID | Pass Kind | Parent Meso-Component | Covered Micro-Features | Assigned Role | Artifact / Code | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| *(Pending)* | - | - | - | - | - | Pending |
