// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Dentry parsing is staged before later refactor passes consume it."
    )
)]

use core::mem::size_of;

use crate::prelude::*;

pub(super) const DENTRY_SIZE: usize = 32;

const EXFAT_UNUSED: u8 = 0x00;
const EXFAT_BITMAP: u8 = 0x81;
const EXFAT_UPCASE: u8 = 0x82;
const EXFAT_VOLUME_LABEL: u8 = 0x83;
const EXFAT_FILE: u8 = 0x85;
const EXFAT_STREAM: u8 = 0xC0;
const EXFAT_NAME: u8 = 0xC1;
const EXFAT_VENDOR_EXT: u8 = 0xE0;
const EXFAT_VENDOR_ALLOC: u8 = 0xE1;

const EXFAT_FILE_NAME_LEN: usize = 15;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct RawExfatDentry {
    pub(super) dentry_type: u8,
    pub(super) value: [u8; 31],
}

const _: [(); DENTRY_SIZE] = [(); size_of::<RawExfatDentry>()];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExfatDentry {
    File(ExfatFileDentry),
    Stream(ExfatStreamDentry),
    Name(ExfatNameDentry),
    Bitmap(ExfatBitmapDentry),
    Upcase(ExfatUpcaseDentry),
    VendorExt(ExfatVendorExtDentry),
    VendorAlloc(ExfatVendorAllocDentry),
    GenericPrimary(ExfatGenericPrimaryDentry),
    GenericSecondary(ExfatGenericSecondaryDentry),
    Deleted(ExfatDeletedDentry),
    Unused,
}

impl ExfatDentry {
    pub(super) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::File(dentry) => dentry.as_bytes(),
            Self::Stream(dentry) => dentry.as_bytes(),
            Self::Name(dentry) => dentry.as_bytes(),
            Self::Bitmap(dentry) => dentry.as_bytes(),
            Self::Upcase(dentry) => dentry.as_bytes(),
            Self::VendorExt(dentry) => dentry.as_bytes(),
            Self::VendorAlloc(dentry) => dentry.as_bytes(),
            Self::GenericPrimary(dentry) => dentry.as_bytes(),
            Self::GenericSecondary(dentry) => dentry.as_bytes(),
            Self::Deleted(dentry) => dentry.as_bytes(),
            Self::Unused => &[0; DENTRY_SIZE],
        }
    }

    pub(super) fn is_volume_label(&self) -> bool {
        matches!(
            self,
            Self::GenericPrimary(dentry) if dentry.dentry_type == EXFAT_VOLUME_LABEL
        )
    }
}

impl From<RawExfatDentry> for ExfatDentry {
    fn from(dentry: RawExfatDentry) -> Self {
        let dentry_bytes = dentry.as_bytes();

        if dentry.dentry_type == EXFAT_FILE {
            return Self::File(ExfatFileDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_STREAM {
            return Self::Stream(ExfatStreamDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_NAME {
            return Self::Name(ExfatNameDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_BITMAP {
            return Self::Bitmap(ExfatBitmapDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_UPCASE {
            return Self::Upcase(ExfatUpcaseDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_VENDOR_EXT {
            return Self::VendorExt(ExfatVendorExtDentry::from_bytes(dentry_bytes));
        }
        if dentry.dentry_type == EXFAT_VENDOR_ALLOC {
            return Self::VendorAlloc(ExfatVendorAllocDentry::from_bytes(dentry_bytes));
        }

        match dentry.dentry_type {
            EXFAT_UNUSED => Self::Unused,
            0x01..=0x7F => Self::Deleted(ExfatDeletedDentry::from_bytes(dentry_bytes)),
            0x80..=0xBF => {
                Self::GenericPrimary(ExfatGenericPrimaryDentry::from_bytes(dentry_bytes))
            }
            0xC2..=0xDF | 0xE2..=0xFF => {
                Self::GenericSecondary(ExfatGenericSecondaryDentry::from_bytes(dentry_bytes))
            }
            _ => unreachable!("all possible entry types are covered"),
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatFileDentry {
    pub(super) dentry_type: u8,
    pub(super) num_secondary: u8,
    pub(super) checksum: u16,
    pub(super) attribute: u16,
    pub(super) reserved1: u16,
    pub(super) create_time: u16,
    pub(super) create_date: u16,
    pub(super) modify_time: u16,
    pub(super) modify_date: u16,
    pub(super) access_time: u16,
    pub(super) access_date: u16,
    pub(super) create_time_cs: u8,
    pub(super) modify_time_cs: u8,
    pub(super) create_utc_offset: u8,
    pub(super) modify_utc_offset: u8,
    pub(super) access_utc_offset: u8,
    pub(super) reserved2: [u8; 7],
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatFileDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatStreamDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) reserved1: u8,
    pub(super) name_len: u8,
    pub(super) name_hash: u16,
    pub(super) reserved2: u16,
    pub(super) valid_size: u64,
    pub(super) reserved3: u32,
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatStreamDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatNameDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) unicode_0_14: [u16; EXFAT_FILE_NAME_LEN],
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatNameDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatBitmapDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) reserved: [u8; 18],
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatBitmapDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatUpcaseDentry {
    pub(super) dentry_type: u8,
    pub(super) reserved1: [u8; 3],
    pub(super) checksum: u32,
    pub(super) reserved2: [u8; 12],
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatUpcaseDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatVendorExtDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) vendor_guid: [u8; 16],
    pub(super) vendor_defined: [u8; 14],
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatVendorExtDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatVendorAllocDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) vendor_guid: [u8; 16],
    pub(super) vendor_defined: [u8; 2],
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatVendorAllocDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatGenericPrimaryDentry {
    pub(super) dentry_type: u8,
    pub(super) secondary_count: u8,
    pub(super) checksum: u16,
    pub(super) flags: u16,
    pub(super) custom_defined: [u8; 14],
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatGenericPrimaryDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatGenericSecondaryDentry {
    pub(super) dentry_type: u8,
    pub(super) flags: u8,
    pub(super) custom_defined: [u8; 18],
    pub(super) start_cluster: u32,
    pub(super) size: u64,
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatGenericSecondaryDentry>()];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod)]
pub(super) struct ExfatDeletedDentry {
    pub(super) dentry_type: u8,
    pub(super) reserved: [u8; 31],
}
const _: [(); DENTRY_SIZE] = [(); size_of::<ExfatDeletedDentry>()];

#[cfg(ktest)]
mod tests {
    use core::mem::size_of;

    use ostd::prelude::ktest;

    use super::*;

    fn raw_dentry(dentry_type: u8) -> RawExfatDentry {
        RawExfatDentry {
            dentry_type,
            value: [0; 31],
        }
    }

    // Confirms the raw on-disk dentry wrapper stays exactly one exFAT entry wide.
    #[ktest]
    fn raw_dentry_has_expected_size() {
        assert_eq!(size_of::<RawExfatDentry>(), DENTRY_SIZE);
    }

    // Confirms the decoder preserves the special concrete entry kinds needed by later parsing.
    #[ktest]
    fn typed_decode_recognizes_special_entry_kinds() {
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_FILE)),
            ExfatDentry::File(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_STREAM)),
            ExfatDentry::Stream(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_NAME)),
            ExfatDentry::Name(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_BITMAP)),
            ExfatDentry::Bitmap(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_UPCASE)),
            ExfatDentry::Upcase(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_VENDOR_EXT)),
            ExfatDentry::VendorExt(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_VENDOR_ALLOC)),
            ExfatDentry::VendorAlloc(_)
        ));
    }

    // Confirms deleted, unused, and generic fallback entries stay distinct from the special kinds.
    #[ktest]
    fn typed_decode_handles_deleted_unused_and_generic_fallbacks() {
        assert!(matches!(
            ExfatDentry::from(raw_dentry(EXFAT_UNUSED)),
            ExfatDentry::Unused
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0x01)),
            ExfatDentry::Deleted(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0x7F)),
            ExfatDentry::Deleted(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0x80)),
            ExfatDentry::GenericPrimary(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0xBF)),
            ExfatDentry::GenericPrimary(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0xC2)),
            ExfatDentry::GenericSecondary(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0xDF)),
            ExfatDentry::GenericSecondary(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0xE2)),
            ExfatDentry::GenericSecondary(_)
        ));
        assert!(matches!(
            ExfatDentry::from(raw_dentry(0xFF)),
            ExfatDentry::GenericSecondary(_)
        ));
    }
}
