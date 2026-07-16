<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Specification: `{component_name}`

*This artifact dictates the dynamic contract for the Meso-Component. Creator Passes must follow it exactly without inventing external architectures or helpers. When a rule applies only to specific micro-features, name those micro-features explicitly so later pass slicing stays deterministic.*

## 1. Modularity (Rely-Guarantee)

### [GUARANTEE] Meso-Level Boundary
*Describe the single semantic crate-visible boundary for this Meso-Component: what class of request enters, what class of result leaves, and what must remain internal control flow beneath that boundary. Do not prescribe an exact Rust function signature, exact type names, or enum / variant spelling unless the packet explicitly says you are documenting an already-fixed pre-existing kernel interface.*
- **Request Class:**
- **Result Class:**
- **Must Remain Internal:**

### [RELY] Bounded Dependencies
*List the explicit OSTD, VFS interfaces, or lower-level capabilities the component is restricted to. Do not use APIs that violate the Architect's lock topology.*
- e.g., `Bio::read_blocks`
- e.g., `VfsTime::now()`

## 2. Functionality (Hoare Logic)

### Pre-conditions
*Logical conditions required of inputs. When applicable, annotate which micro-features depend on each condition.*
- 

### Post-conditions
*Describe the success / failure classes and resulting system state. When applicable, annotate which micro-features each branch covers. You may name semantic cases, but do not invent or freeze exact enum variant spelling unless the packet explicitly authorizes documenting a pre-existing stable interface.*
- **Case 1 (Success):** [Describe the successful result class and resulting state.]
- **Case 2 (Failure Class Y):** [Describe the failure class and any side-effect boundary.]

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
