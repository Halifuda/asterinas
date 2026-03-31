// SPDX-License-Identifier: MPL-2.0

#![cfg(ktest)]

use alloc::fmt::Debug;

use aster_block::{
    bio::{BioEnqueueError, BioStatus, BioType, SubmittedBio},
    BlockDevice, BlockDeviceMeta,
};
use device_id::DeviceId;
use ostd::mm::{io::util::HasVmReaderWriter, FrameAllocOptions, HasSize, Segment, VmIo, PAGE_SIZE};

const BIO_SECTOR_SIZE: usize = 512;

struct ExfatMemoryBioQueue(Segment<()>);

impl ExfatMemoryBioQueue {
    fn new(segment: Segment<()>) -> Self {
        Self(segment)
    }

    fn sectors_count(&self) -> usize {
        self.0.size() / BIO_SECTOR_SIZE
    }
}

pub(super) struct ExfatMemoryDisk {
    queue: ExfatMemoryBioQueue,
}

impl ExfatMemoryDisk {
    fn new(segment: Segment<()>) -> Self {
        Self {
            queue: ExfatMemoryBioQueue::new(segment),
        }
    }

    pub(super) fn read_bytes(&self, offset: usize, bytes: &mut [u8]) {
        self.queue.0.read_bytes(offset, bytes).unwrap();
    }

    pub(super) fn write_bytes(&self, offset: usize, bytes: &[u8]) {
        self.queue.0.write_bytes(offset, bytes).unwrap();
    }
}

impl Debug for ExfatMemoryDisk {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExfatMemoryDisk")
            .field("blocks_count", &self.queue.sectors_count())
            .finish()
    }
}

impl BlockDevice for ExfatMemoryDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::prelude::v1::Result<(), BioEnqueueError> {
        // BIO sector identifiers are fixed 512-byte logical sectors regardless of
        // the exFAT volume's own sector geometry.
        let start_device_ofs = bio.sid_range().start.to_raw() as usize * BIO_SECTOR_SIZE;
        let mut current_device_ofs = start_device_ofs;
        for seg in bio.segments() {
            let size = match bio.type_() {
                BioType::Read => seg
                    .inner_dma()
                    .writer()
                    .unwrap()
                    .write(self.queue.0.reader().skip(current_device_ofs)),
                BioType::Write => self
                    .queue
                    .0
                    .writer()
                    .skip(current_device_ofs)
                    .write(&mut seg.inner_dma().reader().unwrap()),
                _ => 0,
            };
            current_device_ofs += size;
        }

        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        BlockDeviceMeta {
            max_nr_segments_per_bio: usize::MAX,
            nr_sectors: self.queue.sectors_count(),
        }
    }

    fn name(&self) -> &str {
        "exfat-refactor-test-disk"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

static EXFAT_IMAGE: &[u8] = include_bytes!("../../../../../test/initramfs/build/exfat.img");

fn new_vm_segment_from_image() -> Segment<()> {
    // Each ktest gets its own writable copy so mutations stay local to the
    // scenario under test instead of contaminating later tests.
    let segment = FrameAllocOptions::new()
        .zeroed(false)
        .alloc_segment(EXFAT_IMAGE.len().div_ceil(PAGE_SIZE))
        .unwrap();
    segment.write_bytes(0, EXFAT_IMAGE).unwrap();
    segment
}

pub(super) fn load_exfat_disk() -> ExfatMemoryDisk {
    ExfatMemoryDisk::new(new_vm_segment_from_image())
}
