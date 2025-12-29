extern crate alloc;

use crate::{
    drivers::block::nvme::NVMeDevice,
    sync::{Condvar, UPIntrFreeCell},
    DEV_NON_BLOCKING_ACCESS,
};
use alloc::boxed::Box;
use core::error::Error;
use easy_fs::BlockDevice;

/// disable unused warnings for now

#[allow(unused)]
pub struct NVMeBlock {
    nvme_blk: UPIntrFreeCell<NVMeDevice>,
    condvar: Condvar,
}

#[allow(unused)]
impl BlockDevice for NVMeBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            // async
            todo!()
        } else {
            let mut test = self.nvme_blk.exclusive_access();
            // 1 => 512 bytes
            // TODO: there shouldn't be a transfer here
            let data = test.read_sync(1, block_id as u64, 1 as u16);
            match data {
                Ok(data) => {
                    buf.copy_from_slice(&data[..]);
                }
                Err(e) => {
                    panic!("NVMe read error: {:?}", e);
                }
            }
        }
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            // async
            todo!()
        } else {
            // sync
            todo!()
        }
    }

    fn handle_irq(&self) {
        todo!()
    }
}

impl NVMeBlock {
    pub fn new(base_addr: usize) -> Result<Self, Box<dyn Error>> {
        match NVMeDevice::new(base_addr) {
            Ok(nvme) => {
                Ok(NVMeBlock {
                    nvme_blk: unsafe { UPIntrFreeCell::new(nvme) },
                    // theres only on queue to survey, the completion queue
                    condvar: Condvar::new(),
                })
            }
            Err(_) => Err("Failed to create NVMeBlock".into()),
        }
    }
}
