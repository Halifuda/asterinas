// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(dead_code, reason = "File-record helpers are staged before later refactor passes consume them.")
)]

use core::ops::Range;

use crate::prelude::*;

use super::dentry::{
    DENTRY_SIZE, ExfatDentry, ExfatFileDentry, ExfatNameDentry, ExfatStreamDentry,
};

const EXFAT_FILE: u8 = 0x85;
const EXFAT_STREAM: u8 = 0xC0;
const EXFAT_NAME: u8 = 0xC1;
const EXFAT_FILE_NAME_LEN: usize = 15;

#[derive(Debug)]
pub(super) struct ExfatDentrySet {
    dentries: Vec<ExfatDentry>,
}

impl ExfatDentrySet {
    /// Creates a validated file-record set from ordered typed dentries.
    pub(super) fn new(dentries: Vec<ExfatDentry>) -> Result<Self> {
        let dentry_set = Self { dentries };
        dentry_set.validate()?;
        Ok(dentry_set)
    }

    /// Creates a validated file-record set from trusted primary metadata and raw name units.
    pub(super) fn from_trusted_metadata(
        mut file_dentry: ExfatFileDentry,
        mut stream_dentry: ExfatStreamDentry,
        raw_name_units: &[u16],
        mut tail_dentries: Vec<ExfatDentry>,
    ) -> Result<Self> {
        let logical_name_units = logical_name_units(raw_name_units);
        let name_dentries = name_dentries_from_units(logical_name_units);

        let secondary_count = 1usize
            .checked_add(name_dentries.len())
            .and_then(|count| count.checked_add(tail_dentries.len()))
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "file-record is too large"))?;
        let secondary_count = u8::try_from(secondary_count)
            .map_err(|_| Error::with_message(Errno::EINVAL, "file-record is too large"))?;
        let name_len = u8::try_from(logical_name_units.len())
            .map_err(|_| Error::with_message(Errno::EINVAL, "name is too long"))?;

        file_dentry.dentry_type = EXFAT_FILE;
        file_dentry.num_secondary = secondary_count;
        file_dentry.checksum = 0;

        stream_dentry.dentry_type = EXFAT_STREAM;
        stream_dentry.name_len = name_len;
        stream_dentry.name_hash = checksum_utf16(logical_name_units);

        let mut dentries = Vec::with_capacity(secondary_count as usize + 1);
        dentries.push(ExfatDentry::File(file_dentry));
        dentries.push(ExfatDentry::Stream(stream_dentry));
        dentries.extend(name_dentries);
        dentries.append(&mut tail_dentries);

        let checksum = calculate_checksum(&dentries);
        if let ExfatDentry::File(file) = &mut dentries[Self::FILE_INDEX] {
            file.checksum = checksum;
        }

        Self::new(dentries)
    }

    /// Returns the file primary entry by value.
    pub(super) fn file_dentry(&self) -> ExfatFileDentry {
        match self.dentries[Self::FILE_INDEX] {
            ExfatDentry::File(file) => file,
            _ => unreachable!("validated set always stores a file primary first"),
        }
    }

    /// Replaces the file primary entry.
    pub(super) fn set_file_dentry(&mut self, file_dentry: ExfatFileDentry) {
        self.dentries[Self::FILE_INDEX] = ExfatDentry::File(file_dentry);
    }

    /// Returns the stream primary entry by value.
    pub(super) fn stream_dentry(&self) -> ExfatStreamDentry {
        match self.dentries[Self::STREAM_INDEX] {
            ExfatDentry::Stream(stream) => stream,
            _ => unreachable!("validated set always stores a stream primary second"),
        }
    }

    /// Replaces the stream primary entry.
    pub(super) fn set_stream_dentry(&mut self, stream_dentry: ExfatStreamDentry) {
        self.dentries[Self::STREAM_INDEX] = ExfatDentry::Stream(stream_dentry);
    }

    /// Returns the raw UTF-16 name units gathered from the name dentries.
    pub(super) fn raw_name_units(&self) -> Vec<u16> {
        let mut raw_name_units = Vec::with_capacity(self.dentries.len() * EXFAT_FILE_NAME_LEN);
        for dentry in self.name_dentries() {
            for unit in dentry.unicode_0_14 {
                if unit == 0 {
                    return raw_name_units;
                }
                raw_name_units.push(unit);
            }
        }
        raw_name_units
    }

    /// Returns the current bytes of the validated set in on-disk order.
    pub(super) fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.dentries.len() * DENTRY_SIZE);
        for dentry in &self.dentries {
            bytes.extend_from_slice(dentry.as_bytes());
        }
        bytes
    }

    /// Verifies the file-record checksum against the current serialized bytes.
    pub(super) fn verify_checksum(&self) -> bool {
        self.file_dentry().checksum == calculate_checksum(&self.dentries)
    }

    /// Recomputes and stores the file-record checksum.
    pub(super) fn update_checksum(&mut self) {
        let mut file_dentry = self.file_dentry();
        file_dentry.checksum = calculate_checksum(&self.dentries);
        self.dentries[Self::FILE_INDEX] = ExfatDentry::File(file_dentry);
    }

    const FILE_INDEX: usize = 0;
    const STREAM_INDEX: usize = 1;

    fn validate(&self) -> Result<()> {
        if self.dentries.len() > u8::MAX as usize + 1 {
            return_errno_with_message!(Errno::EINVAL, "file-record is too large");
        }

        let Some(first) = self.dentries.first() else {
            return_errno_with_message!(Errno::EINVAL, "file-record is missing the file primary");
        };
        if !matches!(first, ExfatDentry::File(_)) {
            return_errno_with_message!(Errno::EINVAL, "file-record must start with a file primary");
        }

        let Some(second) = self.dentries.get(Self::STREAM_INDEX) else {
            return_errno_with_message!(Errno::EINVAL, "file-record is missing the stream primary");
        };
        if !matches!(second, ExfatDentry::Stream(_)) {
            return_errno_with_message!(Errno::EINVAL, "file-record must place stream second");
        }

        let expected_secondary_count = self.dentries.len() - 1;
        let file_dentry = self.file_dentry();
        if usize::from(file_dentry.num_secondary) != expected_secondary_count {
            return_errno_with_message!(Errno::EINVAL, "secondary count mismatched");
        }

        let mut saw_name = false;
        let mut saw_benign_tail = false;
        for dentry in self.dentries.iter().skip(2) {
            if saw_benign_tail {
                if !is_benign_secondary(dentry) {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "file-record tail must contain only benign secondary entries"
                    );
                }
                continue;
            }

            match dentry {
                ExfatDentry::Name(_) => {
                    saw_name = true;
                }
                entry if is_benign_secondary(entry) => {
                    if !saw_name {
                        return_errno_with_message!(
                            Errno::EINVAL,
                            "file-record must contain at least one name entry"
                        );
                    }
                    saw_benign_tail = true;
                }
                _ => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "file-record contains an unexpected dentry type"
                    );
                }
            }
        }

        if !saw_name {
            return_errno_with_message!(Errno::EINVAL, "file-record must contain a name entry");
        }

        let raw_name_units = self.raw_name_units();
        let stream_dentry = self.stream_dentry();
        if usize::from(stream_dentry.name_len) != raw_name_units.len() {
            return_errno_with_message!(Errno::EINVAL, "name length mismatched");
        }
        if stream_dentry.name_hash != checksum_utf16(&raw_name_units) {
            return_errno_with_message!(Errno::EINVAL, "name hash mismatched");
        }

        if !self.verify_checksum() {
            return_errno_with_message!(Errno::EINVAL, "checksum mismatched");
        }

        Ok(())
    }

    fn name_dentries(&self) -> impl Iterator<Item = ExfatNameDentry> + '_ {
        self.dentries.iter().filter_map(|dentry| match dentry {
            ExfatDentry::Name(name_dentry) => Some(*name_dentry),
            _ => None,
        })
    }
}

fn is_benign_secondary(dentry: &ExfatDentry) -> bool {
    matches!(
        dentry,
        ExfatDentry::GenericSecondary(_)
            | ExfatDentry::VendorExt(_)
            | ExfatDentry::VendorAlloc(_)
    )
}

fn logical_name_units(raw_name_units: &[u16]) -> &[u16] {
    let logical_len = raw_name_units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(raw_name_units.len());
    &raw_name_units[..logical_len]
}

fn name_dentries_from_units(name_units: &[u16]) -> Vec<ExfatDentry> {
    let mut name_dentries = Vec::with_capacity(name_units.len().div_ceil(EXFAT_FILE_NAME_LEN).max(1));

    if name_units.is_empty() {
        name_dentries.push(ExfatDentry::Name(ExfatNameDentry {
            dentry_type: EXFAT_NAME,
            flags: 0,
            unicode_0_14: [0; EXFAT_FILE_NAME_LEN],
        }));
        return name_dentries;
    }

    for chunk in name_units.chunks(EXFAT_FILE_NAME_LEN) {
        let mut unicode_0_14 = [0; EXFAT_FILE_NAME_LEN];
        unicode_0_14[..chunk.len()].copy_from_slice(chunk);
        name_dentries.push(ExfatDentry::Name(ExfatNameDentry {
            dentry_type: EXFAT_NAME,
            flags: 0,
            unicode_0_14,
        }));
    }

    name_dentries
}

fn calculate_checksum(dentries: &[ExfatDentry]) -> u16 {
    const FILE_CHECKSUM_RANGE: Range<usize> = 2..4;
    const EMPTY_RANGE: Range<usize> = 0..0;

    let Some((first, rest)) = dentries.split_first() else {
        return 0;
    };

    let mut checksum = calc_checksum_16(first.as_bytes(), FILE_CHECKSUM_RANGE, 0);
    for dentry in rest {
        checksum = calc_checksum_16(dentry.as_bytes(), EMPTY_RANGE, checksum);
    }
    checksum
}

fn checksum_utf16(units: &[u16]) -> u16 {
    let mut checksum = 0u16;
    for unit in units {
        let [low, high] = unit.to_le_bytes();
        checksum = checksum.rotate_right(1).wrapping_add(low as u16);
        checksum = checksum.rotate_right(1).wrapping_add(high as u16);
    }
    checksum
}

fn calc_checksum_16(data: &[u8], ignore: Range<usize>, prev_checksum: u16) -> u16 {
    let mut checksum = prev_checksum;
    for (index, value) in data.iter().enumerate() {
        if ignore.contains(&index) {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u16::from(*value));
    }
    checksum
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;

    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::dentry::{
        ExfatGenericPrimaryDentry, ExfatVendorExtDentry, RawExfatDentry,
    };

    fn file_dentry(num_secondary: u8, checksum: u16) -> ExfatFileDentry {
        ExfatFileDentry {
            dentry_type: EXFAT_FILE,
            num_secondary,
            checksum,
            attribute: 0,
            reserved1: 0,
            create_time: 0,
            create_date: 0,
            modify_time: 0,
            modify_date: 0,
            access_time: 0,
            access_date: 0,
            create_time_cs: 0,
            modify_time_cs: 0,
            create_utc_offset: 0,
            modify_utc_offset: 0,
            access_utc_offset: 0,
            reserved2: [0; 7],
        }
    }

    fn stream_dentry(name_len: u8, name_hash: u16) -> ExfatStreamDentry {
        ExfatStreamDentry {
            dentry_type: EXFAT_STREAM,
            flags: 0,
            reserved1: 0,
            name_len,
            name_hash,
            reserved2: 0,
            valid_size: 0,
            reserved3: 0,
            start_cluster: 0,
            size: 0,
        }
    }

    fn name_dentry(unicode_0_14: [u16; EXFAT_FILE_NAME_LEN]) -> ExfatDentry {
        ExfatDentry::Name(ExfatNameDentry {
            dentry_type: EXFAT_NAME,
            flags: 0,
            unicode_0_14,
        })
    }

    fn benign_tail_dentry() -> ExfatDentry {
        ExfatDentry::VendorExt(ExfatVendorExtDentry {
            dentry_type: 0xE0,
            flags: 0,
            vendor_guid: [0; 16],
            vendor_defined: [0; 14],
        })
    }

    fn ordered_bytes(dentries: &[ExfatDentry]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(dentries.len() * DENTRY_SIZE);
        for dentry in dentries {
            bytes.extend_from_slice(dentry.as_bytes());
        }
        bytes
    }

    fn decode_bytes(bytes: &[u8]) -> Vec<ExfatDentry> {
        bytes
            .chunks_exact(DENTRY_SIZE)
            .map(|chunk| {
                let raw = RawExfatDentry::from_bytes(chunk);
                ExfatDentry::from(raw)
            })
            .collect()
    }

    // Confirms valid construction keeps checksum, serialization order, and multi-entry name data aligned.
    #[ktest]
    fn fileset_valid_construction_round_trip_serialization() {
        let raw_name_units = vec![
            0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047, 0x0048, 0x0049, 0x004A,
            0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053, 0x0054,
        ];

        let set = ExfatDentrySet::from_trusted_metadata(
            file_dentry(0, 0),
            stream_dentry(0, 0),
            &raw_name_units,
            vec![benign_tail_dentry()],
        )
        .expect("valid file record should construct");

        assert!(set.verify_checksum());
        assert_eq!(set.raw_name_units(), raw_name_units);

        let serialized_bytes = set.to_le_bytes();
        let expected_bytes = ordered_bytes(&set.dentries);
        assert_eq!(serialized_bytes, expected_bytes);
        assert_eq!(decode_bytes(&serialized_bytes), set.dentries);
    }

    // Confirms the raw-name helper concatenates multiple name entries in order and stops at zero padding.
    #[ktest]
    fn fileset_raw_name_aggregation() {
        let mut first_units = [0u16; EXFAT_FILE_NAME_LEN];
        first_units[..EXFAT_FILE_NAME_LEN].copy_from_slice(&[
            0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047, 0x0048, 0x0049, 0x004A,
            0x004B, 0x004C, 0x004D, 0x004E, 0x004F,
        ]);

        let mut second_units = [0u16; EXFAT_FILE_NAME_LEN];
        second_units[..5].copy_from_slice(&[0x0050, 0x0051, 0x0052, 0x0053, 0x0054]);

        let raw_name_units = vec![
            0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047, 0x0048, 0x0049, 0x004A,
            0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053, 0x0054,
        ];

        let entries = vec![
            ExfatDentry::File(file_dentry(3, 0)),
            ExfatDentry::Stream(stream_dentry(20, checksum_utf16(&raw_name_units))),
            name_dentry(first_units),
            name_dentry(second_units),
        ];
        let mut entries = entries;
        let checksum = calculate_checksum(&entries);
        if let ExfatDentry::File(file) = &mut entries[ExfatDentrySet::FILE_INDEX] {
            file.checksum = checksum;
        }

        let mut set = ExfatDentrySet::new(entries).expect("manual file record should validate");
        set.update_checksum();
        assert!(set.verify_checksum());

        assert_eq!(set.raw_name_units(), raw_name_units);
    }

    // Confirms checksum edits stay stale until recomputed and then become valid again.
    #[ktest]
    fn fileset_checksum_update_restores_validity() {
        let mut set = ExfatDentrySet::from_trusted_metadata(
            file_dentry(0, 0),
            stream_dentry(3, 0),
            &[0x0041, 0x0042, 0x0043],
            vec![],
        )
        .expect("valid file record should construct");

        assert!(set.verify_checksum());

        if let ExfatDentry::Stream(stream) = &mut set.dentries[ExfatDentrySet::STREAM_INDEX] {
            stream.name_len = 2;
        } else {
            unreachable!("validated set must store a stream primary second");
        }

        assert!(!set.verify_checksum());
        set.update_checksum();
        assert!(set.verify_checksum());
    }

    // Confirms the constructor rejects malformed ordering and unexpected primaries in the tail.
    #[ktest]
    fn fileset_rejects_malformed_ordering() {
        let valid_tail = vec![benign_tail_dentry()];

        let wrong_first = ExfatDentrySet::new(vec![
            ExfatDentry::Stream(stream_dentry(1, 0)),
            ExfatDentry::File(file_dentry(1, 0)),
        ]);
        assert!(wrong_first.is_err());

        let missing_stream = ExfatDentrySet::new(vec![
            ExfatDentry::File(file_dentry(1, 0)),
            name_dentry({
                let mut units = [0u16; EXFAT_FILE_NAME_LEN];
                units[0] = 0x0041;
                units
            }),
        ]);
        assert!(missing_stream.is_err());

        let name_after_benign = ExfatDentrySet::new(vec![
            ExfatDentry::File(file_dentry(3, 0)),
            ExfatDentry::Stream(stream_dentry(2, 0)),
            name_dentry({
                let mut units = [0u16; EXFAT_FILE_NAME_LEN];
                units[0] = 0x0041;
                units[1] = 0x0042;
                units
            }),
            benign_tail_dentry(),
            name_dentry({
                let mut units = [0u16; EXFAT_FILE_NAME_LEN];
                units[0] = 0x0043;
                units
            }),
        ]);
        assert!(name_after_benign.is_err());

        let unexpected_primary_tail = ExfatDentrySet::new(vec![
            ExfatDentry::File(file_dentry(3, 0)),
            ExfatDentry::Stream(stream_dentry(2, 0)),
            name_dentry({
                let mut units = [0u16; EXFAT_FILE_NAME_LEN];
                units[0] = 0x0041;
                units[1] = 0x0042;
                units
            }),
            ExfatDentry::GenericPrimary(ExfatGenericPrimaryDentry::default()),
        ]);
        assert!(unexpected_primary_tail.is_err());

        assert_eq!(valid_tail.len(), 1);
    }

    // Confirms the checksum guard rejects stale file records instead of repairing them silently.
    #[ktest]
    fn fileset_rejects_checksum_mismatch() {
        let mut set = ExfatDentrySet::from_trusted_metadata(
            file_dentry(0, 0),
            stream_dentry(3, 0),
            &[0x0041, 0x0042, 0x0043],
            vec![],
        )
        .expect("valid file record should construct");

        if let ExfatDentry::File(file) = &mut set.dentries[ExfatDentrySet::FILE_INDEX] {
            file.checksum ^= 0x0001;
        } else {
            unreachable!("validated set must store a file primary first");
        }

        assert!(!set.verify_checksum());
    }
}
