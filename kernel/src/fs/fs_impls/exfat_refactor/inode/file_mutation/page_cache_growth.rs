// SPDX-License-Identifier: MPL-2.0

//! Owns boundary-page preparation for regular-file page-cache growth.
//!
//! Method groups: page-cache boundary preparation.

use core::ops::Range;

use ostd::mm::io::util::HasVmReaderWriter;

use super::super::ExfatInode;
use crate::{
    prelude::*,
    vm::page_cache::PageCache,
};

impl ExfatInode {
    pub(super) fn prepare_regular_file_page_cache_range(
        page_cache: &PageCache,
        current_data_length: usize,
        range: Range<usize>,
    ) -> Result<()> {
        if range.is_empty() {
            return Ok(());
        }

        let vmo = page_cache.as_vmo().clone();
        let prepare_page = |page_idx: usize| -> Result<()> {
            let frame = vmo.commit_on(page_idx)?;
            frame.writer().fill_zeros(PAGE_SIZE);
            Ok(())
        };

        let start_page_idx = range.start / PAGE_SIZE;
        let start_page_offset = start_page_idx
            .checked_mul(PAGE_SIZE)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if !range.start.is_multiple_of(PAGE_SIZE) && start_page_offset >= current_data_length {
            prepare_page(start_page_idx)?;
        }

        if !range.end.is_multiple_of(PAGE_SIZE) {
            let end_page_idx = range.end / PAGE_SIZE;
            let end_page_offset = end_page_idx
                .checked_mul(PAGE_SIZE)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if end_page_offset >= current_data_length
                && (end_page_idx != start_page_idx || range.start.is_multiple_of(PAGE_SIZE))
            {
                prepare_page(end_page_idx)?;
            }
        }
        Ok(())
    }
}
