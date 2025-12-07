use core::fmt::{Display, Formatter};
use core::mem::offset_of;

use alloc::fmt;
use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

use pci_types::{ConfigRegionAccess, EndpointHeader, HeaderType, PciAddress, PciHeader};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::{ReadOnly, ReadWrite};
use tock_registers::register_structs;

use tock_registers::register_bitfields;

use crate::memory::KERNEL_SPACE;



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


pub fn get_pci_base_address(fdt: &fdt::Fdt) -> Result<usize, &'static str> {
    let Some(pci) = fdt.find_compatible(&["pci-host-ecam-generic"]) else {
        info!("No pci-host-ecam-generic controller found");
        return Err("No pci-host-ecam-generic controller found");
    };

    info!("Found PCIe ECAM root: {}", pci.name);

    let reg = pci.reg().unwrap().next().unwrap();
    Ok(reg.starting_address as usize)
}

pub struct Pci {
    base_addr: usize,
}

impl ConfigRegionAccess for Pci {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        unsafe {
            let addr = self.base_addr
                + ((address.bus() as usize) << 20)
                + ((address.device() as usize) << 15)
                + ((address.function() as usize) << 12)
                + (offset as usize);
            core::ptr::read_volatile(addr as *const u32)
        }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        unsafe {
            let addr = self.base_addr
                + ((address.bus() as usize) << 20)
                + ((address.device() as usize) << 15)
                + ((address.function() as usize) << 12)
                + (offset as usize);
            core::ptr::write_volatile(addr as *mut u32, value);
        }
    }
}

fn check_device(bus: u8, device: u8, pci: &Pci) {
    let function = 0;
    let address = PciAddress::new(0, bus, device, function);
    let header = PciHeader::new(address);
    let (vendor_id, device_id) = header.id(pci);
    if vendor_id == 0xFFFF {
        return;
    }
    check_function(bus, device, function, pci);
}

fn check_function(bus: u8, device: u8, function: u8, pci: &Pci) {
    let address = PciAddress::new(0, bus, device, function);
    let header = PciHeader::new(address);
    // (DeviceRevision, BaseClass, SubClass, Interface)
    let (device_revision, base_class, sub_class, interface) =  header.revision_and_class(pci);
    if base_class == 0x01 && sub_class == 0x08 {
        info!("Found NVMe device at {:02x}:{:02x}.{:x}", bus, device, function);
        let entry = header.header_type(pci);
        match entry {
            HeaderType::Endpoint => {
                let mut header = EndpointHeader::from_header(header, pci).unwrap(); 
                info!("  BAR0: {:?}", header.bar(0, pci).unwrap().unwrap_mem());
                info!("  BAR1: {:?}", header.bar(1, pci).unwrap().unwrap_mem());
                // Map BAR0
                
                // setting bar0_addr as MMIO
                unsafe { let _ = header.write_bar(0, pci, 0x00ffe000usize); };
                let (bar0_addr, bar0_size) = header.bar(0, pci).unwrap().unwrap_mem();
                let (bar1_addr, bar1_size) = header.bar(1, pci).unwrap().unwrap_mem();

                // nvme_base_addr = (uint64_t)(((uint64_t)bar1 << 32) | (bar0 & 0xFFFFFFF0));
                let nvme_base_addr = ((bar1_addr as u64) << 32) | ((bar0_addr as u64) & 0xFFFFFFF0);
                info!("  NVMe MMIO Base Address: {:#x}", nvme_base_addr);
                // let nvme_mmio = unsafe { &mut *(nvme_base_addr as *mut NvmeDevice) };
                // info!("  NVMe CAP: {:#x}", nvme_mmio.cap.get());
            }
            _ => {
                info!("  Not a type 0 endpoint");
            }
        }
    }
}

pub fn scan_pci_devices(base_addr: usize) {
    for bus in 0..=255 {
        for device in 0..32 {
            check_device(bus, device, &Pci { base_addr });
        }
    }
}
