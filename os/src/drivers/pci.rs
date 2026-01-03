use alloc::string::{String, ToString};
use hashbrown::HashMap;
use log::info;

// https://pcisig.com/sites/default/files/files/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

use pci_types::{ConfigRegionAccess, EndpointHeader, PciAddress, PciHeader};
use spin::Once;

use crate::{device_tree::DEVICE_TREE_NODES, memory::KERNEL_SPACE};

static PCI_REGISTRY: Once<PciRegistry> = Once::new();

pub struct PciAccess {
    base_addr: usize,
}

#[allow(dead_code)]
pub struct PciDevice {
    base_addr: usize,
    bus: u8,
    device: u8,
    function: u8,
    vendor_id: u16,
    device_id: u16,
    irq_id: u8,
    device_type: PciDeviceType,
}

impl PciDevice {
    pub fn new(
        pci: &PciAccess,
        header: PciHeader,
        address: PciAddress,
        device_type: PciDeviceType,
    ) -> Self {
        let mut endpoint = EndpointHeader::from_header(header, pci).unwrap();
        endpoint.update_command(pci, |command| {
            command
                | pci_types::CommandRegister::BUS_MASTER_ENABLE
                | pci_types::CommandRegister::MEMORY_ENABLE
        });

        let (vendor_id, device_id) = endpoint.header().id(pci);
        let (bus, device, function) = (address.bus(), address.device(), address.function());

        let (irq_pin, _) = endpoint.interrupt(pci);

        // TODO: get addr with a find_free_va(length) instead of hardcoding
        let addr = 0x4000_0000;
        let addr_size = endpoint.bar(0, pci).unwrap().unwrap_mem().1;
        KERNEL_SPACE.exclusive_access().map_mmio(addr, addr_size);
        unsafe {
            match endpoint.write_bar(0, pci, addr) {
                Ok(_) => {
                    let bar0 = endpoint.bar(0, pci).unwrap();
                    return PciDevice {
                        base_addr: bar0.unwrap_mem().0,
                        bus,
                        device,
                        function,
                        vendor_id,
                        device_id,
                        irq_id: DEVICE_TREE_NODES
                            .get()
                            .unwrap()
                            .get_pci()
                            .resolve_pci_irq_id(bus, device, function, irq_pin)
                            .unwrap(),
                        device_type,
                    };
                }
                Err(e) => panic!("Failed to write BAR0: {:?}", e),
            }
        }
    }

    pub fn get_base_addr(&self) -> usize {
        self.base_addr
    }

    pub fn get_irq_id(&self) -> usize {
        self.irq_id as usize
    }

    #[allow(dead_code)]
    pub fn get_device_type(&self) -> &PciDeviceType {
        &self.device_type
    }
}

#[derive(Debug)]
pub enum PciDeviceType {
    NVMe,
    Other,
}

impl From<(u8, u8, u8)> for PciDeviceType {
    fn from(v: (u8, u8, u8)) -> Self {
        match v {
            (0x01, 0x08, 0x02) => PciDeviceType::NVMe,
            _ => PciDeviceType::Other,
        }
    }
}

#[derive(Default)]
pub struct PciRegistry {
    devices: HashMap<String, PciDevice>,
}

impl PciRegistry {
    pub fn init(devices: HashMap<String, PciDevice>) {
        PCI_REGISTRY.call_once(|| PciRegistry { devices });
    }

    pub fn get() -> &'static Self {
        PCI_REGISTRY.get().expect("PCI registry not initialized")
    }

    pub fn device(&self, key: &str) -> Option<&PciDevice> {
        self.devices.get(key)
    }
}

impl ConfigRegionAccess for PciAccess {
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

pub fn scan_pci_devices() {
    let pci_dtn = DEVICE_TREE_NODES.get().unwrap().get_pci();
    let base_addr = pci_dtn.get_base_addr();
    let pci_access: PciAccess = PciAccess { base_addr };

    let mut pci_devices: HashMap<String, PciDevice> = HashMap::new();

    for segment in 0..1 {
        for bus in 0..=255 {
            for device in 0..32 {
                for function in 0..8 {
                    let address = PciAddress::new(segment, bus, device, function);
                    let header = PciHeader::new(address);
                    let (vendor_id, _) = header.id(&pci_access);
                    if vendor_id == 0xFFFF {
                        continue;
                    }
                    let (_, base_class, sub_class, prog_if) =
                        header.revision_and_class(&pci_access);
                    let device_type = PciDeviceType::from((base_class, sub_class, prog_if));
                    match device_type {
                        PciDeviceType::NVMe => {
                            info!(
                                "Found NVMe controller at {:02x}:{:02x}.{:x}",
                                address.bus(),
                                address.device(),
                                address.function()
                            );
                            pci_devices.insert(
                                "nvme0".to_string(),
                                PciDevice::new(&pci_access, header, address, device_type),
                            );
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    PciRegistry::init(pci_devices);
}
