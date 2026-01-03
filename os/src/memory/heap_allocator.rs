extern crate alloc;
use core::cell::UnsafeCell;

use crate::{config::KERNEL_HEAP_SIZE, println};
use buddy_system_allocator::LockedHeap;

#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

struct HeapSpace<const SIZE: usize> {
    buf: UnsafeCell<[u8; SIZE]>,
}
impl<const SIZE: usize> HeapSpace<SIZE> {
    pub const fn new() -> Self {
        HeapSpace {
            buf: UnsafeCell::new([0; SIZE]),
        }
    }

    pub const fn size(&self) -> usize {
        SIZE
    }

    pub const fn as_ptr(&self) -> *mut u8 {
        self.buf.get() as *mut u8
    }
}
unsafe impl<const SIZE: usize> Sync for HeapSpace<SIZE> {}

static HEAP_SPACE: HeapSpace<KERNEL_HEAP_SIZE> = HeapSpace::new();

/// initiate heap allocator
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, HEAP_SPACE.size());
    }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as *const () as usize..ebss as *const () as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for (i, val) in v.iter().take(500).enumerate() {
        assert_eq!(*val, i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
