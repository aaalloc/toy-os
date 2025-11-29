extern crate alloc;
use core::panic;
use core::ptr::NonNull;

use super::BlockDevice;
use crate::board::VirtAddrEnum;
use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{Condvar, UPIntrFreeCell};
use crate::task::schedule;
use crate::DEV_NON_BLOCKING_ACCESS;
use alloc::collections::BTreeMap;

use log::{info, warn};
use virtio_drivers::device::blk::{BlkReq, BlkResp, VirtIOBlk};
use virtio_drivers::transport::mmio::{self, MmioTransport, VirtIOHeader};
use virtio_drivers::transport::Transport;

pub struct VirtIOBlock<'a> {
    virtio_blk: UPIntrFreeCell<VirtIOBlk<VirtioHal, MmioTransport<'a>>>,
    condvars: BTreeMap<u16, Condvar>,
}

impl<'a> BlockDevice for VirtIOBlock<'static> {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut req = BlkReq::default();
            let mut resp = BlkResp::default();
            let mut token = 0u16;
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                token = unsafe {
                    blk.read_blocks_nb(block_id, &mut req, buf, &mut resp)
                        .unwrap()
                };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });

            schedule(task_cx_ptr);
            unsafe {
                self.virtio_blk
                    .exclusive_session(|blk| {
                        blk.complete_write_blocks(token, &mut req, buf, &mut resp)
                    })
                    .expect("Error when writing VirtIOBlk");
            }
        } else {
            self.virtio_blk
                .exclusive_access()
                .read_blocks(block_id, buf)
                .expect("VirtIOBlk read error");
        }
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut req = BlkReq::default();
            let mut resp = BlkResp::default();
            let mut token = 0u16;
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                token = unsafe {
                    blk.write_blocks_nb(block_id, &mut req, &buf, &mut resp)
                        .unwrap()
                };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });

            schedule(task_cx_ptr);
            unsafe {
                self.virtio_blk
                    .exclusive_session(|blk| {
                        blk.complete_write_blocks(token, &mut req, &buf, &mut resp)
                    })
                    .expect("Error when writing VirtIOBlk");
            }
        } else {
            self.virtio_blk
                .exclusive_access()
                .write_blocks(block_id, &buf)
                .expect("VirtIOBlk write error");
        }
    }

    fn handle_irq(&self) {
        self.virtio_blk.exclusive_session(|blk| {
            if let Some(token) = blk.peek_used() {
                self.condvars.get(&token).unwrap().signal();
            }
        });
    }
}

impl VirtIOBlock<'_> {
    pub fn new() -> Self {
        let virtio_blk = {
            let mmio_size = 0x00_1000;
            let header = NonNull::new(VirtAddrEnum::VIRTIO as *mut VirtIOHeader).unwrap();
            let transport = match unsafe { MmioTransport::new(header, mmio_size) } {
                Err(e) => {
                    warn!("Error creating VirtIO MMIO transport: {}", e);
                    panic!("Failed to create VirtIO blk transport");
                }
                Ok(transport) => {
                    info!(
                        "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                        transport.vendor_id(),
                        transport.device_type(),
                        transport.version(),
                    );
                    transport
                }
            };

            VirtIOBlk::<VirtioHal, mmio::MmioTransport>::new(transport)
                .expect("Failed to create VirtIO blk")
        };

        let queue_size = virtio_blk.virt_queue_size();

        let mut condvars = BTreeMap::new();
        for i in 0..queue_size {
            condvars.insert(i as u16, Condvar::new());
        }

        Self {
            virtio_blk: unsafe { UPIntrFreeCell::new(virtio_blk) },
            condvars,
        }
    }
}
