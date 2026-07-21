<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Validation Contract: `{component_name}`

*This artifact is not a test-design document. It is the sole external-evidence contract for the `Checker`, expressed through the xfstests lane. The mapping is many-to-many; missing upstream coverage is recorded as a gap rather than filled with another test lane. This refactor must not create, modify, or grow any ktest or other internal test surface. Any xfstests harness/configuration change must be outside `kernel/src/fs/fs_impls/` and explicitly authorized by the packet.*

## 1. External Validation Mapping

| Micro-Features | Upstream Test IDs / Groups | Expected Observation | Coverage Class |
| :--- | :--- | :--- | :--- |
|  |  |  | `direct` / `combined` / `not-run/unsupported` / `no upstream coverage` |

## 2. Pass-Scoped Checker Obligations

- **Creator-Synced Pass Scope:** The Checker mirrors the Creator Pass parent and micro-feature set exactly.
- **Selected External Tests:**
- **Expected Evidence:** Record exact test IDs/groups, result files, guest logs, filesystem image or remount observations, and `PASS`, `FAIL`, or `NOTRUN` classification.
- **Coverage Limitation:** State which selected tests cover neighboring features and which assigned features are not isolated or covered by the upstream suite. Do not propose another test lane as a substitute.

## 3. Invariant and Integration Observations

### Runtime Observation
- **Related Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**
- **Evidence Limitation:** [State that this is runtime evidence, not an internal memory-safety proof, when applicable.]

### Meso-Level Integration Validation

*Each integration scenario involves tightly coupled micro-features and is implemented as an independent Checker pass. Add optional paths only when an upstream test exists; otherwise state that the upstream lane provides no such scenario and leave the coverage gap explicit.*

### Success Path (Mandatory)
- **Covered Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**
