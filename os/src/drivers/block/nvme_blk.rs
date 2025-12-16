extern crate alloc;

use crate::{
    drivers::block::nvme::NVMeDevice,
    sync::{Condvar, UPIntrFreeCell},
};
use alloc::{boxed::Box, collections::BTreeMap};
use core::error::Error;
use easy_fs::BlockDevice;

/// disable unused warnings for now

#[allow(unused)]
pub struct NVMeBlock {
    nvme_blk: UPIntrFreeCell<NVMeDevice>,
    condvars: BTreeMap<u16, Condvar>,
}

#[allow(unused)]
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
    pub fn new(base_addr: usize) -> Result<Self, Box<dyn Error>> {
        match NVMeDevice::new(base_addr) {
            Ok(mut nvme) => {
                nvme.identify_controller()?;
                Err("sdfkljdsajkfhasfjaskfh".into())
                // Ok(NVMeBlock {
                //     nvme_blk: unsafe { UPIntrFreeCell::new(nvme) },
                //     condvars: BTreeMap::new(),
                // })
            }
            Err(_) => Err("Failed to create NVMeBlock".into()),
        }
    }
}
