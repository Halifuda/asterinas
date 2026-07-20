# SPDX-License-Identifier: MPL-2.0

# Overlayfs uses the two xfstests images as its ext2 base filesystems.
XFSTESTS_NEEDS_BLOCK_DEVICES := true
XFSTESTS_MKFS := mkfs.ext2
