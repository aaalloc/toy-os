extern crate alloc;

use crate::{
    drivers::{
        block::{
            nvme::{Dma, NVMeDevice},
            BlockDeviceTmp,
        },
        plic::PlicDevice,
    },
    sync::{Condvar, UPIntrFreeCell},
    DEV_NON_BLOCKING_ACCESS,
};
use alloc::boxed::Box;
use core::error::Error;
use easy_fs::BlockDevice;

pub struct NVMeBlock {
    nvme_blk: UPIntrFreeCell<NVMeDevice>,
    doing_io: UPIntrFreeCell<bool>,
    irq_id: usize,
    condvar: Condvar,
}

impl BlockDeviceTmp for NVMeBlock {}

#[allow(unused)]
impl PlicDevice for NVMeBlock {
    fn irq_id(&self) -> usize {
        self.irq_id
    }

    fn irq_handler(&self) {
        self.handle_irq();
    }
}

impl BlockDevice for NVMeBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        // log::info!(
        //     "NVMe read_block {} requested for block {}",
        //     if nb { "async" } else { "sync" },
        //     block_id
        // );
        if nb {
            self.doing_io.exclusive_access().clone_from(&true);
            self.nvme_blk
                .exclusive_access()
                .send_io_read(1, block_id as u64, 1 as u16);
            let task_cx_ptr = self.condvar.wait_no_sched();
            crate::task::schedule(task_cx_ptr);
            // TODO: currently handler ACK completion, so we just retrieve data here. This has to be done here
            self.nvme_blk.exclusive_session(|nvme| {
                // TODO: there shouldn't be a transfer here
                let data = nvme.retrieve_dma_buffer(buf.len());
                buf.copy_from_slice(&data[..]);
            });
        } else {
            self.nvme_blk.exclusive_session(|nvme| {
                let mut status = 0u16;
                // 1 => 512 bytes
                nvme.send_io_read(1, block_id as u64, 1 as u16)
                    .io_complete_command(&mut status);
                match status {
                    0 => {
                        // TODO: there shouldn't be a transfer here
                        let data = nvme.retrieve_dma_buffer(buf.len());
                        buf.copy_from_slice(&data[..]);
                    }
                    _ => {
                        panic!("NVMe read_block failed with status: {}", status);
                    }
                }
            });
        }
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        // create dma buffer and copy data
        let mut dma_buf = Dma::<[u8; 1024]>::new().unwrap();
        dma_buf.as_mut_slice()[..buf.len()].copy_from_slice(buf);
        if nb {
            self.doing_io.exclusive_access().clone_from(&true);
            self.nvme_blk.exclusive_access().send_io_write(
                1,
                block_id as u64,
                1 as u16,
                dma_buf.paddr().0,
            );
            let task_cx_ptr = self.condvar.wait_no_sched();
            crate::task::schedule(task_cx_ptr);
            // TODO: currently handler ACK completion, so we just retrieve data here. This has to be done here
        } else {
            self.nvme_blk.exclusive_session(|nvme| {
                let mut status = 0u16;
                // 1 => 512 bytes
                nvme.send_io_write(1, block_id as u64, 1 as u16, dma_buf.paddr().0)
                    .io_complete_command(&mut status);
                match status {
                    0 => {}
                    _ => {
                        panic!("NVMe read_block failed with status: {}", status);
                    }
                }
            });
        }
    }

    fn handle_irq(&self) {
        // NOTE: doing this is I think stupid but from what i've seen, irq is fired before read_block is called
        if self.doing_io.exclusive_access().clone() == false {
            log::warn!("NVMe IRQ received but no IO in progress");
            return;
        }
        let mut status = 0u16;
        self.nvme_blk.exclusive_session(|nvme| {
            nvme.io_complete_command(&mut status);
            match status {
                0 => self.condvar.signal(),
                _ => {
                    panic!("NVMe IRQ handling failed with status: {}", status);
                }
            }
        });
    }
}

impl NVMeBlock {
    pub fn new(base_addr: usize, irq_id: usize) -> Result<Self, Box<dyn Error>> {
        // return Err("bla".into());
        match NVMeDevice::new(base_addr) {
            Ok(nvme) => {
                Ok(NVMeBlock {
                    nvme_blk: unsafe { UPIntrFreeCell::new(nvme) },
                    doing_io: unsafe { UPIntrFreeCell::new(false) },
                    irq_id,
                    // theres only on queue to survey, the completion queue
                    condvar: Condvar::new(),
                })
            }
            Err(_) => Err("Failed to create NVMeBlock".into()),
        }
    }
}
