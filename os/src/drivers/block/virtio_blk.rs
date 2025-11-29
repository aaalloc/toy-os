extern crate alloc;
use core::ptr::NonNull;

use super::BlockDevice;
use crate::board::VirtAddrEnum;
use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{Condvar, UPIntrFreeCell};
use crate::task::schedule;
use crate::DEV_NON_BLOCKING_ACCESS;
use alloc::collections::BTreeMap;

use easy_fs::BLOCK_SZ;
use log::{info, warn};
use virtio_drivers::device::blk::{BlkReq, BlkResp, RespStatus, VirtIOBlk};
use virtio_drivers::transport::mmio::{self, MmioTransport, VirtIOHeader};
use virtio_drivers::transport::Transport;

pub struct PendingReq {
    pub req: BlkReq,
    pub buf: [u8; BLOCK_SZ],
    pub resp: alloc::boxed::Box<BlkResp>,
    pub cond: Condvar,
}

pub struct VirtIOBlock<'a> {
    virtio_blk: UPIntrFreeCell<VirtIOBlk<VirtioHal, MmioTransport<'a>>>,
    // token: u16, req: &BlkReq, buf: &mut [u8], resp: &mut BlkResp
    condvars: BTreeMap<u16, Condvar>,
}

impl<'a> BlockDevice for VirtIOBlock<'static> {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut resp = BlkResp::default();
            let mut req = BlkReq::default();
            let token = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe {
                    blk.read_blocks_nb(block_id, &mut req, buf, &mut resp)
                        .unwrap()
                };
                token
            });
            let task_cx_ptr = {
                // let mut pending_buf = [0u8; BLOCK_SZ];
                // pending_buf[..buf.len()].copy_from_slice(buf);
                // let mut pending_req: &mut PendingReq = self.condvars.get(&token).unwrap();
                // // pending_req.buf.copy_from_slice(&pending_buf);
                // // pending_req.req = req;
                // pending_req.resp = alloc::boxed::Box::new(resp);
                self.condvars.get(&token).unwrap().wait_no_sched()
            }; // NOTE: could not be copied because virtio_blk exclusive session

            schedule(task_cx_ptr);
            match resp.status() {
                RespStatus::OK => (),
                _ => panic!("VirtIOBlk read error"),
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
            let mut resp = BlkResp::default();
            let mut req = BlkReq::default();
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe {
                    blk.write_blocks_nb(block_id, &mut req, buf, &mut resp)
                        .unwrap()
                };

                self.condvars.get(&token).unwrap().wait_no_sched()
            });

            schedule(task_cx_ptr);

            match resp.status() {
                RespStatus::OK => (),
                _ => panic!("VirtIOBlk write error"),
            }
        } else {
            self.virtio_blk
                .exclusive_access()
                .write_blocks(block_id, buf)
                .expect("VirtIOBlk write error");
        }
    }

    fn handle_irq(&self) {
        self.virtio_blk.exclusive_session(|blk| {
            while let Some(token) = blk.peek_used() {
                let pending_req = self.condvars.get(&token).unwrap();
                // unsafe {
                //     blk.complete_read_blocks(
                //         token,
                //         pending_req.req.as_ref(),
                //         &mut pending_req.buf[..],
                //         pending_req.resp.as_mut(),
                //     )
                // };
                // pending_req.condvar.signal();
                // blk.complete_read_blocks(token);
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
