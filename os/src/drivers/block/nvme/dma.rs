#![allow(dead_code)]
/// thanks to https://github.com/rcore-os/virtio-drivers/blob/2e0beb35631e89742b3665104b7a08b521a15f2c/src/hal.rs#L91
use crate::{
    config::PAGE_SIZE,
    memory::{
        frame_alloc_more, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr,
        PhysPageNum, StepByOne, VirtAddr,
    },
    sync::UPIntrFreeCell,
};
use alloc::{fmt, vec::Vec};
use core::ops::{Index, IndexMut};
use core::{marker::PhantomData, ptr::NonNull};
use lazy_static::lazy_static;

lazy_static! {
    static ref QUEUE_FRAMES: UPIntrFreeCell<Vec<FrameTracker>> =
        unsafe { UPIntrFreeCell::new(Vec::new()) };
}

/// A region of contiguous physical memory used for DMA.
#[derive(Debug)]
pub struct Dma<T> {
    paddr: PhysAddr,
    vaddr: NonNull<T>,
    pages: usize,
    _marker: PhantomData<T>,
}

// SAFETY: DMA memory can be accessed from any thread.
unsafe impl<T> Send for Dma<T> {}

// SAFETY: `&Dma` only allows pointers and physical addresses to be returned. Any actual access to
// the memory requires unsafe code, which is responsible for avoiding data races.
unsafe impl<T> Sync for Dma<T> {}
impl<T> Dma<T> {
    /// Allocates the given number of pages of physically contiguous memory to be used for DMA in
    /// the given direction.
    ///
    /// The pages will be zeroed.
    pub fn new() -> Result<Self, DMAError> {
        let bytes = size_of::<T>();
        assert!(bytes > 0);

        let pages = (bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        let (paddr, vaddr) = Self::dma_alloc(pages);
        if paddr == 0 {
            return Err(DMAError::AllocationFailed);
        }
        Ok(Self {
            paddr: PhysAddr::from(paddr as usize),
            vaddr: vaddr.cast(),
            pages,
            _marker: PhantomData,
        })
    }

    /// Returns the physical address of the start of the DMA region, as seen by devices.
    pub fn paddr(&self) -> PhysAddr {
        self.paddr
    }

    /// Returns a pointer to the given offset within the DMA region.
    pub fn vaddr(&self, offset: usize) -> NonNull<u8> {
        assert!(offset < self.pages * PAGE_SIZE);
        NonNull::new((self.vaddr.as_ptr() as usize + offset) as _).unwrap()
    }

    /// Returns a pointer to the entire DMA region as a slice.
    pub fn raw_slice(&self) -> NonNull<[u8]> {
        let raw_slice =
            core::ptr::slice_from_raw_parts_mut(self.vaddr(0).as_ptr(), self.pages * PAGE_SIZE);
        NonNull::new(raw_slice).unwrap()
    }

    fn dma_alloc(pages: usize) -> (u64, NonNull<u8>) {
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

    unsafe fn dma_dealloc(&self, paddr: u64, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr as usize);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    unsafe fn mmio_phys_to_virt(&self, paddr: u64, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as *mut u8).unwrap()
    }

    unsafe fn share(&self, buffer: NonNull<[u8]>) -> u64 {
        let ptr = buffer.as_non_null_ptr().as_ptr() as usize;
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(ptr))
            .unwrap()
            .0 as u64
    }

    unsafe fn unshare(&self, _paddr: u64, _buffer: NonNull<[u8]>) {}
}

impl<T> Drop for Dma<T> {
    fn drop(&mut self) {
        // SAFETY: The memory was previously allocated by `dma_alloc` in `Dma::new`,
        // not yet deallocated, and we are passing the values from then.
        let err = unsafe { self.dma_dealloc(self.paddr.0 as u64, self.vaddr.cast(), self.pages) };
        assert_eq!(err, 0, "failed to deallocate DMA");
    }
}

impl<T, const N: usize> Dma<[T; N]> {
    pub fn as_slice(&self) -> &[T] {
        unsafe { &self.vaddr.as_ref()[..] }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut self.vaddr.as_mut()[..] }
    }
}

impl<T, const N: usize> Index<usize> for Dma<[T; N]> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < N);
        unsafe { &self.vaddr.as_ref()[index] }
    }
}

impl<T, const N: usize> IndexMut<usize> for Dma<[T; N]> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < N);
        unsafe { &mut self.vaddr.as_mut()[index] }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DMAError {
    AllocationFailed,
}

impl fmt::Display for DMAError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DMAError::AllocationFailed => write!(f, "DMA allocation failed"),
        }
    }
}

impl core::error::Error for DMAError {}
