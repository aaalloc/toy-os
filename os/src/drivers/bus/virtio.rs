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
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (u64, NonNull<u8>) {
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

    unsafe fn dma_dealloc(paddr: u64, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr as usize);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: u64, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as *mut u8).unwrap()
    }
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> u64 {
        let ptr = buffer.as_non_null_ptr().as_ptr() as usize;
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(ptr))
            .unwrap()
            .0 as u64
    }

    unsafe fn unshare(_paddr: u64, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}
