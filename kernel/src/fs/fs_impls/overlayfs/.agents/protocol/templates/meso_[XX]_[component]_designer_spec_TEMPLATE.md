<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso-Component Designer Specification: `{component_name}`

*This artifact dictates the dynamic contract and the concrete Rust code form for the Meso-Component. Creator Passes must implement it exactly, without inventing external architectures or helpers. When a rule applies only to specific micro-features, name those micro-features explicitly so later pass slicing stays deterministic. Signature/type/helper design must follow the coding guidelines: `priors/ASTERINAS_CODE_QUALITY_PRIORS.md` and `book/src/to-contribute/coding-guidelines/` (see `PROTOCOL.md` §0.5).*

## 1. Modularity (Rely-Guarantee)

### [GUARANTEE] Meso-Level Boundary
*Describe the single crate-visible Rust boundary for this Meso-Component in concrete signature form: the entry structs/traits, their method signatures (arguments, return types, error types), the carrier types that cross the boundary, and what must remain internal control flow beneath that boundary. Pre-existing stable kernel interfaces (VFS traits, OSTD primitives) are inherited constraints — cite them exactly. New names must satisfy the coding guidelines' naming and visibility rules.*
- **Request Class (entry types/signatures):**
- **Result Class (return/error types):**
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
*Describe the success / failure classes and resulting system state. When applicable, annotate which micro-features each branch covers. Freeze the semantic cases AND their Rust representation: the enum/error types and variants (or success-result carriers) that encode each case.*
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


## 4. Rust Code-Form Design (Mandatory Signature Design)

*Produce the complete meso-level Rust surface the Creator will implement.
This section is NOT advisory. Follow the coding guidelines
(`priors/ASTERINAS_CODE_QUALITY_PRIORS.md` and
`book/src/to-contribute/coding-guidelines/`): naming conventions, narrowest
visibility, checked/saturating arithmetic, no `.unwrap()`/`.expect()` in
production paths, owner-private helpers unless a stable meso entry / trait /
cross-owner / invariant rule admits them.*

- **Module Layout:** [module path and file/module split with rationale]

### Structs / Carriers
*[For each new struct: fields with types, the invariant each field protects,
the owner/guard boundary that justifies it.]*
- 

### Enums
*[For each new enum: variants with names/spellings and the invariant encoded;
closed sets prefer `enum` over trait objects.]*
- 

### Helper Signatures
*[For each new internal helper: signature + one-line purpose, or the explicit
reason it is inlined into an owner method.]*
- 

### Lock Carriers
*[Which struct fields carry which lock domains (`DIR`, `CUL`, `INODE`, `WL`,
or the reserved `UPPER`/`WL` cleanup candidates), with sleep-capability
constraints (`Mutex` for BIO-capable domains).]*
- 

### Naming / Style Compliance Confirmation
*[Confirm the proposed names satisfy the priors' conventions; fix any name the
guidelines would reject in this spec rather than deferring it.]*
- 

### Complexity Baseline
*[Advisory counts for new entities, long-parameter functions, temporary
carriers, coordination objects, or repeated spec text, and deliberate budget
overruns. Every named intermediate type must be listed here with a one-line
justification; pure temporaries are locals, not types.]*
- 

### Revision Disposition
*[For a revision continuation: changed obligations, preserved obligations, and
any Architect escalation.]*
- 
