<!-- SPDX-License-Identifier: MPL-2.0 -->

# Macro 00: Global Topology

## 1. Identified Macro-Owners
<!-- List the primary architectural domains in the system (e.g., ExfatFs, ExfatInode) -->
- `MacroOwnerName`: Brief description of what this macro-domain owns and its lifecycle scope.
- ...

## 2. Global Lock Topology & Hierarchy
<!-- Define the absolute static hierarchy of macro-level locks to prevent locking cycles globally. -->
- **Level 1 (Top Level)**: `LockName` (e.g., `ExfatFs allocator lock`)
- **Level 2**: `LockName` (e.g., `ExfatInode local rw_lock`)
- **Level 3 (Lowest)**: `LockName`

> **Hierarchy Rule:** A thread holding a Level N lock MAY NOT acquire a Level M lock if M <= N.

## 3. Structural Invariants
<!-- Any other macro-level lifetime or structural invariant requirements (e.g., VFS final owner Drop order constraints). -->
