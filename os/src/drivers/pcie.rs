use core::fmt::{Display, Formatter};
use core::mem::offset_of;

use alloc::fmt;
use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

use pci_types::{ConfigRegionAccess, EndpointHeader, HeaderType, PciAddress, PciHeader};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

use tock_registers::register_bitfields;
use virtio_drivers::device;

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
                | ((address.bus() as usize) << 20)
                | ((address.device() as usize) << 15)
                | ((address.function() as usize) << 12)
                | (offset as usize);
            core::ptr::read_volatile(addr as *const u32)
        }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        unsafe {
            let addr = self.base_addr
                | ((address.bus() as usize) << 20)
                | ((address.device() as usize) << 15)
                | ((address.function() as usize) << 12)
                | (offset as usize);
            core::ptr::write_volatile(addr as *mut u32, value);
        }
    }
}

pub fn nvme_setup(endpoint: &mut EndpointHeader, pci: &Pci, address: PciAddress) {
    endpoint.update_command(&pci, |command| {
        command
            | pci_types::CommandRegister::BUS_MASTER_ENABLE
            | pci_types::CommandRegister::MEMORY_ENABLE
    });

    let cap = endpoint.capabilities(&pci);

    let (vendor_id, device_id) = endpoint.header().id(pci);
    let (bus, device, function) = (address.bus(), address.device(), address.function());
    info!(
        "-> Found NVMe device: {:04x}:{:04x} at {:02x}:{:02x}.{:x}",
        vendor_id, device_id, bus, device, function
    );
    for c in cap {
        info!("  -> Capability: {:?}", c);
    }
    // TODO: select last addr instead of getting that and getting length also from reading bar
    let addr = 0x4000_0000;
    KERNEL_SPACE.exclusive_access().map_mmio(addr, 0x4000);
    unsafe {
        match endpoint.write_bar(0, &pci, addr) {
            Ok(_) => {}
            Err(e) => {
                info!("Failed to write BAR0: {:?}", e);
            }
        }
    };
    let bar0 = endpoint.bar(0, &pci).unwrap();
    info!("  -> BAR0 address: {:?}", bar0);
    let addr_bar0 = bar0.unwrap_mem().0;
    let nvme_dev = unsafe { &mut *(addr_bar0 as *mut NvmeDevice) };
    let cap = nvme_dev.cap.get();
    info!("  -> NVMe CAP: 0x{:x}", cap);
    info!("    -> MQES: {}", (cap & 0xFFFF) + 1);
    info!("    -> CQR: {}", (cap >> 16) & 0x1);
    info!("    -> AMS: {}", (cap >> 17) & 0x7);
    info!("    -> TO: {}", (cap >> 24) & 0xFF);
    info!("    -> DSTRD: {}", (cap >> 32) & 0xF);
    info!("    -> NVMSET: {}", (cap >> 48) & 0xFFFF);

    // get serial device
    let version = nvme_dev.vs.get();
    info!(
        "  -> NVMe Version: {}.{}.{}",
        (version >> 16) & 0xFF,
        (version >> 8) & 0xFF,
        version & 0xFF
    );
    // More NVMe initialization would go here...
}

pub fn scan_pci_devices(base_addr: usize) {
    for segment in 0..1 {
        for bus in 0..=255 {
            for device in 0..32 {
                for function in 0..8 {
                    let pci = Pci { base_addr };
                    let address = PciAddress::new(segment, bus, device, function);
                    let header = PciHeader::new(address);
                    let (vendor_id, device_id) = header.id(&pci);
                    if vendor_id == 0xFFFF {
                        continue;
                    }
                    let (device_revision, base_class, sub_class, interface) =
                        header.revision_and_class(&pci);
                    // check if not nvme
                    match &header.header_type(&pci) {
                        HeaderType::Endpoint => {
                            info!(
                                "Found PCI Endpoint: {:04x}:{:04x} at {:02x}:{:02x}.{:x} - class: {:02x}, subclass: {:02x}, interface: {:02x}",
                                vendor_id, device_id, bus, device, function, base_class, sub_class, interface
                            );
                            let mut endpoint = EndpointHeader::from_header(header, &pci).unwrap();
                            if base_class == 0x01 && sub_class == 0x08 && interface == 0x02 {
                                nvme_setup(&mut endpoint, &pci, address);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
