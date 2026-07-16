<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Validation Contract: `{component_name}`

*This artifact defines the externally observable validation obligations for the `Checker`. It must separate Creator-synced validation from independent meso-level integration validation. The default expected lane is NixOS-driven xfstests unless the upstream project standardizes another filesystem-validation path. Do not request new filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`.*

## 1. Creator-Synced Validation Obligations

### Validation Scenario: Base-Case Success
- **Related Micro-Features:**
- **Upstream Lane:** [e.g., NixOS xfstests generic group/test IDs, or another upstream-approved lane.]
- **Setup:**
- **Execution Chain:** Exercise the single exported Meso-Level Interface through the mounted filesystem or approved system-level path.
- **Assertion / Receipt:** [Expected xfstests result, guest log condition, filesystem state, or equivalent receipt.]

### Validation Scenario: Error Paths
- **Related Micro-Features:**
- **Scenario [Error Variant X]:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**

## 2. Invariant / Rollback Observations

*Validation required to certify memory safety, structural coherence (e.g., FAT chain linkage), and rollback stability. These obligations may be satisfied in Creator-synced passes when they map cleanly to the covered micro set.*

### Observation Scenario 1
- **Related Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**

## 3. Meso-Level Integration Validation

*Each integration scenario must involve tightly coupled micro-features and is implemented as an independent Checker pass. The `Success Path` entry is mandatory whenever the meso-component has more than trivial cross-micro interaction. The other three path types are optional depending on complexity; if omitted, explain why.*

### Success Path (Mandatory)
- **Covered Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**

### Failure-Maintenance Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**

### Idempotence / Repeated-Call Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**

### Concurrency Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Upstream Lane:**
- **Setup:**
- **Execution Chain:**
- **Assertion / Receipt:**
