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
            // sync
            todo!()
        }
        // if nb {
        //     let mut req = BlkReq::default();
        //     let mut resp = BlkResp::default();
        //     let mut token = 0u16;
        //     let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
        //         token = unsafe {
        //             blk.read_blocks_nb(block_id, &mut req, buf, &mut resp)
        //                 .unwrap()
        //         };
        //         self.condvars.get(&token).unwrap().wait_no_sched()
        //     });

        //     schedule(task_cx_ptr);
        //     unsafe {
        //         self.virtio_blk
        //             .exclusive_session(|blk| {
        //                 blk.complete_write_blocks(token, &mut req, buf, &mut resp)
        //             })
        //             .expect("Error when writing VirtIOBlk");
        //     }
        // } else {
        //     self.virtio_blk
        //         .exclusive_access()
        //         .read_blocks(block_id, buf)
        //         .expect("VirtIOBlk read error");
        // }
        todo!()
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
            Ok(mut nvme) => {
                nvme.identify_controller()?;
                let ns = nvme.identify_namespace_list(0);
                for n in ns {
                    log::info!("ns_id: {n}");
                    nvme.identify_namespace(n);
                }
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
