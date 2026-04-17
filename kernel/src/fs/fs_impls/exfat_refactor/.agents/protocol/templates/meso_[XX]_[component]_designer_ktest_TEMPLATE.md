<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer KTest: `{component_name}`

*This artifact defines the exact testing obligations for the `Checker`. It must separate Creator-synced unit obligations from independent meso-level integration obligations. Each scenario should explain `Setup`, `Execution Chain`, and `Assertion` at a high level only, without line-by-line implementation detail.*

## 1. Creator-Synced Unit Test Obligations

### Unit Scenario: Base-Case Success
- **Related Micro-Features:**
- **Setup:**
- **Execution Chain:** Call the single exported Meso-Level Interface.
- **Assertion:** 

### Unit Scenario: Error Paths
- **Related Micro-Features:**
- **Scenario [Error Variant X]:**
- **Setup:**
- **Execution Chain:**
- **Assertion:** 

## 2. Invariant / Rollback Obligations

*Tests required to certify memory safety, structural coherence (e.g., FAT chain linkage), and rollback stability. These obligations may be implemented in Creator-synced passes when they map cleanly to the covered micro set.*
### Invariant Scenario 1
- **Related Micro-Features:**
- **Setup:**
- **Execution Chain:**
- **Assertion:** 

## 3. Meso-Level Integration Test Obligations

*Each integration scenario must involve tightly coupled micro-features and is implemented as an independent Checker pass. The `Success Path` entry is mandatory whenever the meso-component has more than trivial cross-micro interaction. The other three path types are optional depending on complexity; if omitted, explain why.*

### Success Path (Mandatory)
- **Covered Micro-Features:**
- **Setup:**
- **Execution Chain:**
- **Assertion:**

### Failure-Maintenance Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Setup:**
- **Execution Chain:**
- **Assertion:**

### Idempotence / Repeated-Call Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Setup:**
- **Execution Chain:**
- **Assertion:**

### Concurrency Path (Optional)
- **Required?:** [Yes/No + one-line reason]
- **Covered Micro-Features:**
- **Setup:**
- **Execution Chain:**
- **Assertion:**
