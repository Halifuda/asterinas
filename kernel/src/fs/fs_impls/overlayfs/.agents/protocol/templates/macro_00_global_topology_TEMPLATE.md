<!-- SPDX-License-Identifier: MPL-2.0 -->

# Macro 00: Global Topology

## 1. Identified On-disk Structure Owners
<!-- List the concrete durable filesystem structures or state machines that act as stable physical truths. -->
- `OnDiskStructureOwnerName`: Brief description of the durable structure, state machine, and persistence scope.
- ...

## 2. Identified Runtime Owners
<!-- List the runtime authorities that coordinate or project the durable truths into the running filesystem model. -->
- `RuntimeOwnerName`: Brief description of the runtime authority, lifecycle scope, and primary responsibility.
- ...

## 3. On-disk Structure Owner -> Runtime Owner Projection
<!-- Map each durable truth source to its primary runtime authority. Record secondary collaborators only when they materially affect later static boundaries. -->
| On-disk Structure Owner | Primary Runtime Owner | Secondary Runtime Owner(s) / Notes | Why this projection exists |
|---|---|---|---|
| e.g. `StreamExtension` | `Inode(file)` | `Filesystem` for global coordination notes | File stream state is projected through the file inode runtime authority |

## 4. Candidate Meso-Component Index
<!-- Generate the first meso candidates from the ownership projection. A candidate meso should already be large enough for one Architect traceability map and one Designer contract. -->
| Candidate Meso-Component | Primary Runtime Owner | Entry-Surface Family | Durable Touch-Set | Static Lock Envelope | Why this is one meso boundary |
|---|---|---|---|---|---|
| e.g. `file_content_mutation` | `Inode(file)` | `write` / `truncate` / `extend` | `StreamExtension`, `AllocationMap`, `FAT`, file entry-set metadata | `InodeRwLock(Write)` + lower-level allocator locks only | These features share one content-mutation sequencing and failure domain |

## 5. Global Lock Topology & Hierarchy
<!-- Define the absolute static hierarchy of macro-level locks only after the owner and candidate-meso structure is explicit, so locking validates the structure instead of substituting for it. -->
- **Level 1 (Top Level)**: `LockName` (e.g., `Filesystem allocator lock`)
- **Level 2**: `LockName` (e.g., `Inode local rw_lock`)
- **Level 3 (Lowest)**: `LockName`

> **Hierarchy Rule:** A thread holding a Level N lock MAY NOT acquire a Level M lock if M <= N.

## 6. Structural Invariants
<!-- Any other macro-level lifetime or structural invariant requirements (e.g., VFS final owner Drop order constraints, owner-gap prohibitions, or cross-cutting overlay rules that stay above individual meso contracts). -->
