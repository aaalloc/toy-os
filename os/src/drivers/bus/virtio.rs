extern crate alloc;
use core::ptr::NonNull;

use crate::memory::{
    frame_alloc_more, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use crate::sync::UPIntrFreeCell;
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{BufferDirection, Hal};

lazy_static! {
    static ref QUEUE_FRAMES: UPIntrFreeCell<Vec<FrameTracker>> =
        unsafe { UPIntrFreeCell::new(Vec::new()) };
}

pub struct VirtioHal;

unsafe impl Hal for VirtioHal {
    // fn dma_alloc(pages: usize) -> usize {
    //     let trakcers = frame_alloc_more(pages);
    //     let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
    //     QUEUE_FRAMES
    //         .exclusive_access()
    //         .append(&mut trakcers.unwrap());
    //     let pa: PhysAddr = ppn_base.into();
    //     pa.0
    // }

    // fn dma_dealloc(pa: usize, pages: usize) -> i32 {
    //     let pa = PhysAddr::from(pa);
    //     let mut ppn_base: PhysPageNum = pa.into();
    //     for _ in 0..pages {
    //         frame_dealloc(ppn_base);
    //         ppn_base.step();
    //     }
    //     0
    // }

    // fn phys_to_virt(addr: usize) -> usize {
    //     addr
    // }

    // fn virt_to_phys(vaddr: usize) -> usize {
    //     PageTable::from_token(kernel_token())
    //         .translate_va(VirtAddr::from(vaddr))
    //         .unwrap()
    //         .0
    // }

    fn dma_alloc(pages: usize, direction: BufferDirection) -> (u64, NonNull<u8>) {
        let trakcers = frame_alloc_more(pages);
        let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .exclusive_access()
            .append(&mut trakcers.unwrap());
        let pa: PhysAddr = ppn_base.into();
        (
            pa.0.try_into().unwrap(),
            NonNull::new(pa.0 as *mut u8).unwrap(),
        )
    }

    unsafe fn dma_dealloc(paddr: u64, vaddr: NonNull<u8>, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr as usize);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: u64, size: usize) -> NonNull<u8> {
        NonNull::new(paddr as *mut u8).unwrap()
    }
    unsafe fn share(buffer: NonNull<[u8]>, direction: BufferDirection) -> u64 {
        let ptr = buffer.as_non_null_ptr().as_ptr() as usize;
        let len = buffer.len();

        // Identity-mapped: virtual == physical
        let pa = ptr;

        // Optional: flush/invalidate caches here depending on direction
        // match direction {
        //     BufferDirection::DeviceToDriver => flush_dcache(ptr, len),
        //     BufferDirection::DriverToDevice => invalidate_dcache(ptr, len),
        //     BufferDirection::Bidirectional => {
        //         flush_dcache(ptr, len);
        //         invalidate_dcache(ptr, len);
        //     }
        // }

        pa as u64
    }

    unsafe fn unshare(paddr: u64, buffer: NonNull<[u8]>, direction: BufferDirection) {
        let ptr = buffer.as_non_null_ptr().as_ptr() as usize;
        let len = buffer.len();

        // Optional: cache maintenance here
        // match direction {
        //     BufferDirection::DeviceToDriver => invalidate_dcache(ptr, len),
        //     BufferDirection::DriverToDevice => flush_dcache(ptr, len),
        //     BufferDirection::Bidirectional => {
        //         flush_dcache(ptr, len);
        //         invalidate_dcache(ptr, len);
        //     }
        // }

        // Nothing else to do in identity-mapped systems.
    }
}
