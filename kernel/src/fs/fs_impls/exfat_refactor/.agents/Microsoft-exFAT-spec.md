# exFAT File System Specification Summary

**Context**: This document is a core knowledge base for LLM Agents and low-level system developers regarding the exFAT file system. Redundant human-oriented descriptions are omitted. It uses a high-density, structured, and deterministic format, highlighting memory structures, offsets, strict constraints, and checksum algorithms to allow LLMs to directly extract rules when writing parsers or recovering data.

## 0. Design Baseline
*   **Endianness**: Little-Endian.
*   **Character Set**: Unicode (UTF-16LE), no null-terminator limitation.
*   **Starting Cluster Number**: The first valid cluster in the Data Region is strictly `2`.
*   **Cluster Size**: $2^{\text{SectorsPerClusterShift}} \times 2^{\text{BytesPerSectorShift}}$ bytes, up to a maximum of 32 MB.

---

## 1. Volume Layout
The physical disk space is strictly divided into 4 contiguous regions:

| Region Name | Start Offset (Sectors) | Length (Sectors) | Description |
| :--- | :--- | :--- | :--- |
| **Main Boot Region** | `0` | `12` | Contains BPB, extended boot code, OEM parameters, and checksum. |
| **Backup Boot Region**| `12` | `12` | Exact replica of the Main Boot Region. |
| **FAT Region** | `FatOffset` | `FatLength * NumberOfFats` | File Allocation Table area (typically only 1 FAT). |
| **Data Region** | `ClusterHeapOffset`| `ClusterCount * (2^SectorsPerClusterShift)` | Cluster Heap, containing all files and directories. |

---

## 2. Boot Sector Data Structure (Main/Backup BPB)
Located at Sector 0 and Sector 12. Parsers **MUST** verify the `BootSignature` and pass the `BootChecksum` before mounting.

| Offset (Hex) | Field Name | Length (Bytes)| Valid Values / Constraints |
| :--- | :--- | :--- | :--- |
| `0x00` | JumpBoot | 3 | MUST be `EB 76 90`. |
| `0x03` | FileSystemName | 8 | MUST be `"EXFAT   "` (with three trailing spaces). |
| `0x0B` | MustBeZero | 53 | MUST be all `0` (overwrites legacy FAT32 BPB fields). |
| `0x40` | PartitionOffset | 8 | Physical sector offset of this partition (used by BIOS ints). |
| `0x48` | VolumeLength | 8 | Total number of sectors in the volume. |
| `0x50` | FatOffset | 4 | Sector index of the first FAT ($\ge 24$). |
| `0x54` | FatLength | 4 | Number of sectors occupied by one FAT. |
| `0x58` | ClusterHeapOffset| 4 | Starting sector index of the Data Region (Cluster Heap). |
| `0x5C` | ClusterCount | 4 | Total number of clusters ($\le 2^{32} - 11$). |
| `0x60` | FirstClusterOfRootDirectory| 4 | Starting cluster of the Root Directory. |
| `0x64` | VolumeSerialNumber| 4 | Volume serial number (typically a timestamp hash). |
| `0x68` | FileSystemRevision| 2 | Version: Low byte Minor (`0x00`), High byte Major (`0x01`). |
| `0x6A` | VolumeFlags | 2 | See **[Volume Flags Definition]** below. |
| `0x6C` | BytesPerSectorShift| 1 | $\log_2(\text{Bytes per Sector})$, valid: `9` (512B) to `12` (4KB). |
| `0x6D` | SectorsPerClusterShift| 1 | $\log_2(\text{Sectors per Cluster})$, range: `0` to `25 - BytesPerSectorShift`. |
| `0x6E` | NumberOfFats | 1 | `1` or `2` (2 is strictly for TexFAT). |
| `0x6F` | DriveSelect | 1 | Typically `0x80`. |
| `0x70` | PercentInUse | 1 | Percentage of allocated clusters (`0`-`100`), or `0xFF` (unavailable). |
| `0x71` | Reserved | 7 | MUST be `0`. |
| `0x78` | BootCode | 390| Boot code (if none, fill with `0xF4` HLT instructions). |
| `0x1FE`| BootSignature | 2 | MUST be `0xAA55`. |

### 2.1 Volume Flags Definition (Offset `0x6A`)
*   **Bit 0 (`ActiveFat`)**: 0 = First FAT active; 1 = Second FAT active (TexFAT only).
*   **Bit 1 (`VolumeDirty`)**: 1 = Volume is in an inconsistent state (requires Chkdsk).
*   **Bit 2 (`MediaFailure`)**: 1 = Underlying media has reported bad blocks.
*   **Bit 3 (`ClearToZero`)**: Parsers MUST clear this bit to `0` before modifying the volume.

---

## 3. FAT & Allocation Bitmap Mechanisms

### 3.1 FAT Status Values
The FAT only records cluster chains for fragmented files or system structures. Each entry is **4 Bytes**.
*   `FatEntry[0]` = `0xFFFFFFF8` (Media Type Marker)
*   `FatEntry[1]` = `0xFFFFFFFF` (Reserved)
*   `FatEntry[X]` = Next cluster number (Range: $2$ to $ClusterCount + 1$)
*   **Special Markers**:
    *   `0xFFFFFFF7` = Bad Cluster
    *   `0xFFFFFFFF` = End of File (EOF / End of Chain)

### 3.2 Allocation Bitmap
exFAT tracks free space using a Bitmap instead of scanning the FAT.
*   Exists as a special hidden file within the Data Region.
*   **Bit `0`**: Cluster is Free; **Bit `1`**: Cluster is Allocated or Bad.
*   The lowest-order bit (Bit 0) of the first byte represents Cluster `2`.

---

## 4. Directory and Directory Entry Structures
A directory is an array of **32-Byte Directory Entries**. A logical file or folder is described by a **Directory Entry Set**: `[1 Primary Entry] + [N Secondary Entries]`.

### 4.1 EntryType (Byte `0`) Parsing Rules
`EntryType` defines the nature of the 32B block.
*   **`0x00`**: End-Of-Directory (EOD). The directory ends here. All subsequent bytes in the cluster are invalid. Parsers MUST stop scanning immediately.
*   **`0x01` ~ `0x7F`**: InUse = `0` (Deleted/Free entry). Parsers MUST skip this 32B block linearly.
*   **`0x81` ~ `0xFF`**: InUse = `1` (Valid entry).

**EntryType Bit-Level Breakdown**:
*   `Bit 7` (InUse): 1 = In Use, 0 = Free/Deleted.
*   `Bit 6` (TypeCategory): 0 = Primary, 1 = Secondary.
*   `Bit 5` (TypeImportance): 0 = Critical (throw error if unrecognized), 1 = Benign (ignore safely).
*   `Bits 0-4` (TypeCode): Specific type identifier.

### 4.2 Core Entry Types (Hex)
| EntryType | Name | Category | Importance | Description |
| :---: | :--- | :---: | :---: | :--- |
| `0x81`| **Allocation Bitmap** | Primary | Critical | Pointer to the global bitmap (Root Dir only). |
| `0x82`| **Up-case Table** | Primary | Critical | Pointer to the Up-case mapping table (Root Dir only). |
| `0x83`| **Volume Label** | Primary | Critical | Volume name (Root Dir only). |
| `0x85`| **File** | Primary | Critical | Main descriptor for a file/directory. |
| `0xC0`| **Stream Extension** | Secondary| Critical | Physical location/size (Must follow `0x85`). |
| `0xC1`| **File Name** | Secondary| Critical | Filename chunks (Must follow `0xC0`). |

---

## 5. Key Directory Entry Payload Structures

### 5.1 File Entry (`0x85`)
Describes basic file attributes and timestamps.
*   `0x01` SecondaryCount (1): Number of subsequent entries in this Set (Stream + Names).
*   `0x02` SetChecksum (2): Checksum for the entire Directory Entry Set.
*   `0x04` FileAttributes (2): DOS attributes (`0x01`=Read-Only, `0x02`=Hidden, `0x04`=System, `0x10`=Directory, `0x20`=Archive).
*   `0x08` CreateTimestamp (4) / `0x0C` LastModifiedTimestamp (4) / `0x10` LastAccessedTimestamp (4).

### 5.2 Stream Extension Entry (`0xC0`)
Describes the absolute physical location and state of the data (**CRITICAL**).
*   `0x01` GeneralSecondaryFlags (1):
    *   **Bit 0 (`AllocationPossible`)**: 1 = Allocation possible.
    *   **Bit 1 (`NoFatChain`)**: **Core Mechanism!** If `1`, file data is **absolutely contiguous** on disk. Parsers MUST NOT query the FAT table; calculate sectors directly from `FirstCluster` and `DataLength`. If `0`, parsers MUST traverse the FAT chain.
*   `0x03` NameLength (1): Total character count of the filename.
*   `0x04` NameHash (2): Hash of the up-cased filename (for rapid collision checks).
*   `0x08` ValidDataLength (8): Amount of valid data written. (For directories, MUST equal `DataLength`).
*   `0x14` FirstCluster (4): Starting cluster number of the data.
*   `0x18` DataLength (8): Total allocated size in bytes (max `16 EB`).

### 5.3 File Name Entry (`0xC1`)
Stores the UTF-16LE file name.
*   `0x01` GeneralSecondaryFlags (1): Ignored.
*   `0x02` FileName (30): Stores up to **15** Unicode characters.
*   *Note*: If `NameLength > 15`, multiple contiguous `0xC1` entries are used. Any unused character space in the final entry MUST be padded with `0x0000`.

---

## 6. Mandatory Algorithms

LLM Agents MUST implement the following algorithms when generating or validating exFAT metadata (C-Style pseudocode provided):

### 6.1 Boot Sector Checksum
Scope: The first 11 sectors of the Main Boot Region.
```c
uint32_t BootChecksum(uint8_t sectors[11 * BytesPerSector]) {
    uint32_t checksum = 0;
    for (int i = 0; i < 11 * BytesPerSector; i++) {
        // Skip VolumeFlags (bytes 106, 107) and PercentInUse (byte 112)
        if (i == 106 || i == 107 || i == 112) continue;
        checksum = ((checksum & 1) ? 0x80000000 : 0) + (checksum >> 1) + sectors[i];
    }
    return checksum;
}
```

### 6.2 Directory Entry Set Checksum
Scope: Starts from the `0x85` (File Entry) and covers all its Secondary Entries (based on `SecondaryCount`). The File Entry's own checksum field is skipped during calculation.
```c
uint16_t EntrySetChecksum(uint8_t* entry_set, uint8_t secondary_count) {
    uint16_t checksum = 0;
    int num_bytes = (secondary_count + 1) * 32;
    for (int i = 0; i < num_bytes; i++) {
        // Skip the SetChecksum field itself (offsets 2 and 3 of the Primary Entry)
        if (i == 2 || i == 3) continue;
        checksum = ((checksum & 1) ? 0x8000 : 0) + (checksum >> 1) + entry_set[i];
    }
    return checksum;
}
```

### 6.3 Name Hash
Scope: Used for `NameHash` in the `0xC0` entry. Input MUST be the **Up-cased** filename.
```c
uint16_t NameHash(uint16_t* upcased_filename, uint8_t name_length) {
    uint16_t hash = 0;
    uint8_t* buffer = (uint8_t*)upcased_filename;
    for (int i = 0; i < name_length * 2; i++) {
        hash = ((hash & 1) ? 0x8000 : 0) + (hash >> 1) + buffer[i];
    }
    return hash;
}
```

---

## 7. Parser Rules and Constraints (For LLM Agents)

When reasoning about, writing to, or extracting data from an exFAT image, strictly adhere to the following logic:

1.  **Handling `NoFatChain`**: If `NoFatChain == 1` in the Stream Extension, you are strictly forbidden from traversing or modifying the FAT table for this file. Total clusters are calculated mathematically: `TotalClusters = ceil(DataLength / ClusterSize)`.
2.  **Handling Deletions and "Holes"**: To delete a file, change the highest bit of Byte `0` from `1` to `0` for ALL entries in the Set (e.g., `0x85` becomes `0x05`, `0xC0` becomes `0x40`). **DO NOT** rely on the `SecondaryCount` of a deleted file to skip over holes. If `InUse == 0`, you MUST increment your parser pointer linearly by exactly 32 Bytes per iteration.
3.  **Physical Addressing Formula**: To locate the physical sector of Cluster $N$:
    `PhysicalSector = ClusterHeapOffset + (N - 2) * (2 ^ SectorsPerClusterShift)`
4.  **Case-Insensitivity & Up-case Table**: exFAT is strictly case-insensitive. When searching for a file, you MUST convert both your target string and the extracted `FileName` string using the exact Up-case Table stored on the volume before comparing.
5.  **The `0x00` EOD Terminator**: While linearly scanning 32B directory blocks, if you encounter `Byte 0 == 0x00`, it guarantees that the current directory ends immediately. You MUST NOT continue scanning the current cluster or follow the FAT chain to the next directory cluster.
