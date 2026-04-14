<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso XX: <Component_Name> Architecture

## 1. Meso-Component Definition
- **Component**: `<Component_Name>` (e.g., `write_at`)
- **Macro-Owner**: `<Target_Macro_Owner>` (e.g., `ExfatInode`)
- **Responsibility**: A brief description of this specific component.

## 2. Micro-Feature Traceability Matrix
<!-- List ALL micro-features from the inventory mapped to this component. NO OWNER GAPS ALLOWED. -->
| Micro-Feature Name | Prior Reference | Description / Requisite |
|---|---|---|
| e.g., `Zero-fill gap` | Linux exFAT / Issue #0413 | Must fill uninitialized space with zeros when extending file size |
| e.g., `Update Mtime` | Spec 7.4.5 | Update modification time on write success |

## 3. Static Lock Boundaries
- **Expected Inlet State**:
  <!-- What locks MUST already be held by the caller before entering this component? -->
  - Ex: `Requires InodeRwLock(Write)` / `Requires No Locks`
- **Topology Placement**:
  <!-- Where does this component legally sit in the macro_00_global_topology hierarchy? -->
  - Highest lock level permitted to acquire internally: `Level N`
  - Prohibited dependencies: `Cannot acquire any lock above Level N`

## 4. External structural interactions
<!-- Static, strict interactions with other Macro components. 
DO NOT write dynamic execution paths. 
DO NOT advise on private helper function architectures (leave to Creator). -->
