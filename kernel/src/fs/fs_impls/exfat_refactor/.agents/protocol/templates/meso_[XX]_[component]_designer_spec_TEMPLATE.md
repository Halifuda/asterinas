<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Specification: `{component_name}`

*This artifact dictates the dynamic contract for the Meso-Component. Creator Passes must follow it exactly without inventing external architectures or helpers. When a rule applies only to specific micro-features, name those micro-features explicitly so later pass slicing stays deterministic.*

## 1. Modularity (Rely-Guarantee)

### [GUARANTEE] Meso-Level Interface
*The singular, strict public or crate-visible Rust function signature.*
```rust
// e.g., pub(crate) fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize, ExfatError>
```

### [RELY] Bounded Dependencies
*List the explicit OSTD, VFS interfaces, or lower-level capabilities the component is restricted to. Do not use APIs that violate the Architect's lock topology.*
- e.g., `Bio::read_blocks`
- e.g., `VfsTime::now()`

## 2. Functionality (Hoare Logic)

### Pre-conditions
*Logical conditions required of inputs. When applicable, annotate which micro-features depend on each condition.*
- 

### Post-conditions
*Exact success outcomes and defined error variants mapping. When applicable, annotate which micro-features each branch covers.*
- **Case 1 (Success):** Returns `Ok(...)`, resulting in state X.
- **Case 2 (Error Condition Y):** Returns `Err(...)`, zero side-effects.

### Invariants
*Integrity rules spanning the execution. When applicable, annotate which micro-features each invariant protects.*
- 

## 3. Dynamic Lock Orchestration

### Inlet/Outlet Lock State
*Inherited from Architect. What static state must the system be in when this executes?*
- **Inlet:** 
- **Outlet:**

### Acquisition Order
*If local locks within the Meso-Component must be acquired, specify the topological order.*
1. 
2. 

### Concurrency & Non-blocking Hazards
*State the specific blocking points (e.g., calling `Bio`) and handoffs. Mandate that no deadlocking locks be held across these points.*
- **Hazard 1:** 
