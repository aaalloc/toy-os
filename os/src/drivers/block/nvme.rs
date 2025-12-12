extern crate alloc;

use easy_fs::BlockDevice;
use tock_registers::{
    interfaces::Readable,
    register_structs,
    registers::{ReadOnly, ReadWrite},
};

use crate::sync::{Condvar, UPIntrFreeCell};
use alloc::collections::BTreeMap;

/// disable unused warnings for now

#[allow(unused)]
pub struct NVMeBlock {
    nvme_blk: UPIntrFreeCell<u128>,
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

#[allow(unused)]
impl NVMeBlock {
    pub fn new() -> Self {
        // first, we need to get PCIE, for that need to check file device tree
        // NOTE: NVMe controllers can be found as PCI devices with class code 1 and subclass code 8.
        // let nvme = Nvme::new(bar, config);
        todo!()
    }
}

#[allow(unused)]
pub struct ControllerInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub sqes_max: u8,
    pub sqes_min: u8,
    pub cqes_max: u8,
    pub cqes_min: u8,
    pub max_cmd: u16,
    pub number_of_namespaces: u32,
}

register_structs! {

    // https://wiki.osdev.org/NVMe
    pub NvmeDevice {
        (0x00 => pub cap: ReadOnly<u64>),        // Controller Capabilities
        (0x08 => pub vs: ReadOnly<u32>),         // Version
        (0x0C => pub intms: ReadWrite<u32>),      // Interrupt Mask Set
        (0x10 => pub intmc: ReadWrite<u32>),      // Interrupt Mask Clear
        (0x14 => pub cc: ReadWrite<u32>),         // Controller Configuration
        (0x18 => _rsvd1: [u8; 4]),
        (0x1C => pub csts: ReadOnly<u32>),       // Controller Status
        (0x20 => pub nssr: ReadWrite<u32>),       // NVM Subsystem Reset (optional)
        (0x24 => pub aqa: ReadWrite<u32>),        // Admin Queue Attributes
        (0x28 => pub asq: ReadWrite<u64>),        // Admin Submission Queue Base Address
        (0x30 => pub acq: ReadWrite<u64>),        // Admin Completion Queue Base Address
        // NOTE: not sure
        (0x38 => pub cmbloc: ReadWrite<u32>),     // Controller Memory Buffer Location (optional)
        (0x3C => pub cmbsz: ReadWrite<u32>),      // Controller Memory Buffer Size (optional)
        (0x40 => pub bpinfo: ReadWrite<u32>),     // Boot Partition Information
        (0x44 => pub bprsel: ReadWrite<u32>),     // Boot Partition Read Select
        (0x48 => pub bpmbl: ReadWrite<u64>),      // Boot Partition Memory Buffer Location
        (0x50 => @END),
    }

}

pub struct NVMeController {
    nvme_dev: &'static mut NvmeDevice,
}

#[allow(unused)]
impl NVMeController {
    pub fn new(addr_ptr: usize) -> Self {
        let nvme_dev = unsafe { &mut *(addr_ptr as *mut NvmeDevice) };
        NVMeController { nvme_dev }
    }

    pub fn init(&mut self) {
        // Initialize the NVMe controller
        // For example, set up admin queues, enable the controller, etc.
        // This is a placeholder for actual initialization code.
        todo!()
    }

    pub fn version(&self) -> (u8, u8, u8) {
        let version = self.nvme_dev.vs.get();
        (
            ((version >> 16) & 0xFF) as u8,
            ((version >> 8) & 0xFF) as u8,
            (version & 0xFF) as u8,
        )
    }
}
