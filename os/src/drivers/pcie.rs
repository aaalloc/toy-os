use alloc::vec::Vec;
use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

use pci_types::{ConfigRegionAccess, EndpointHeader, PciAddress, PciHeader};

use crate::drivers::block::NVMeController;
use crate::memory::KERNEL_SPACE;

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

pub enum PciDevice {
    NVMe(NVMeController),
    Other,
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

pub fn nvme_setup(pci: &Pci, header: PciHeader, address: PciAddress) -> usize {
    let mut endpoint = EndpointHeader::from_header(header, pci).unwrap();
    endpoint.update_command(pci, |command| {
        command
            | pci_types::CommandRegister::BUS_MASTER_ENABLE
            | pci_types::CommandRegister::MEMORY_ENABLE
    });

    let (vendor_id, device_id) = endpoint.header().id(pci);
    let (bus, device, function) = (address.bus(), address.device(), address.function());
    info!(
        "-> Found NVMe device: {:04x}:{:04x} at {:02x}:{:02x}.{:x}",
        vendor_id, device_id, bus, device, function
    );
    // TODO: select last addr instead of getting that and getting length also from reading bar
    let addr = 0x4000_0000;
    KERNEL_SPACE.exclusive_access().map_mmio(addr, 0x4000);
    unsafe {
        match endpoint.write_bar(0, &pci, addr) {
            Ok(_) => {
                let bar0 = endpoint.bar(0, &pci).unwrap();
                return bar0.unwrap_mem().0;
            }
            Err(e) => panic!("Failed to write BAR0: {:?}", e),
        }
    };
}

pub fn scan_pci_devices(base_addr: usize) -> Vec<PciDevice> {
    let mut devices = Vec::<PciDevice>::new();
    for segment in 0..1 {
        for bus in 0..=255 {
            for device in 0..32 {
                for function in 0..8 {
                    let pci = Pci { base_addr };
                    let address = PciAddress::new(segment, bus, device, function);
                    let header = PciHeader::new(address);
                    let (vendor_id, _) = header.id(&pci);
                    if vendor_id == 0xFFFF {
                        continue;
                    }
                    let (_, base_class, sub_class, prog_if) = header.revision_and_class(&pci);
                    match (base_class, sub_class, prog_if) {
                        (0x01, 0x08, 0x02) => {
                            info!(
                                "Found NVMe controller at {:02x}:{:02x}.{:x}",
                                address.bus(),
                                address.device(),
                                address.function()
                            );
                            let nvme_base_addr = nvme_setup(&pci, header, address);
                            let mut nvme = NVMeController::new(nvme_base_addr);
                            nvme.init();
                            devices.push(PciDevice::NVMe(nvme));
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    devices
}
