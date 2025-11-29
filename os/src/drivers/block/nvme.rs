extern crate alloc;

use easy_fs::BlockDevice;
use nvme_driver::Nvme;

use crate::sync::{Condvar, UPIntrFreeCell};
use alloc::collections::BTreeMap;

pub struct NVMeBlock {
    nvme_blk: UPIntrFreeCell<u128>,
    condvars: BTreeMap<u16, Condvar>,
}

impl BlockDevice for NVMeBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        todo!()
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        todo!()
    }

    fn handle_irq(&self) {
        todo!()
    }
}

impl NVMeBlock {
    pub fn new() -> Self {
        // first, we need to get PCIE, for that need to check file device tree
        // NOTE: NVMe controllers can be found as PCI devices with class code 1 and subclass code 8.
        // let nvme = Nvme::new(bar, config);
        todo!()
    }
}
