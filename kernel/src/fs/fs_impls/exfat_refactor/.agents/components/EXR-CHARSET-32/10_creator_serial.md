# EXR-CHARSET-32 Serial Creator Artifact

## Implemented Boundary

- Added owner-private `ConvertedName` and `ConvertedLabel` value types under `ExfatFs` in `fs.rs`.
- Added `ExfatFs::convert_name()` and `ExfatFs::convert_label()` to validate external `&str` input and materialize validated UTF-16 units.
- Added `ExfatFs::visible_name_from_utf16_units()` so read-side callers decode validated on-disk UTF-16 through the filesystem owner instead of using local `String::from_utf16()`.
- Migrated `ExfatInode::lookup()` to use the `ExfatFs` name-conversion boundary instead of local `encode_utf16()`.
- Migrated `ExfatInode::readdir_at()` to use the `ExfatFs` visible-name decode boundary instead of local `String::from_utf16()`.

## Notes

- Fold and hash ownership remains in the existing `ExfatFs` upcase helpers.
- No namespace mutation, volume-label mutation, or low-level file-record constructor changes were introduced.
- The new helper surface remains owner-local to `ExfatFs` and is intended to be reused by later namespace and volume-label work.

