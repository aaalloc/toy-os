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

pub struct PciDevice {
    base_addr: usize,
    bus: u8,
    device: u8,
    function: u8,
    irq_id: usize,
    device_type: PciDeviceType,
}

impl PciDevice {
    pub fn new(
        base_addr: usize,
        bus: u8,
        device: u8,
        function: u8,
        interrupt_pin: u8,
        device_type: PciDeviceType,
    ) -> Self {
        // let irq_id = DEVICE_TREE_NODES
        //     .get()
        //     .unwrap()
        //     .get_pci()
        //     .resolve_pci_irq_id(bus, device, function, interrupt_pin)
        //     .unwrap();
        PciDevice {
            base_addr,
            bus,
            device,
            function,
            irq_id: 2,
            device_type,
        }
    }

    pub fn get_base_addr(&self) -> usize {
        self.base_addr
    }

    pub fn get_irq_id(&self) -> usize {
        self.irq_id
    }
}

#[derive(Debug)]
pub enum PciDeviceType {
    NVMe,
    Other,
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

fn nvme_setup(pci: &PciAccess, header: PciHeader, address: PciAddress) -> (usize, usize) {
    let mut endpoint = EndpointHeader::from_header(header, pci).unwrap();
    endpoint.update_command(pci, |command| {
        command
            | pci_types::CommandRegister::BUS_MASTER_ENABLE
            | pci_types::CommandRegister::MEMORY_ENABLE
    });

    let (vendor_id, device_id) = endpoint.header().id(pci);
    let (bus, device, function) = (address.bus(), address.device(), address.function());

    // TODO: register interupt line or pin so that we can register it to plic
    let (pin, line) = endpoint.interrupt(pci);
    info!("   Interrupt Line: {}", line);
    info!("   Interrupt Pin: {}", pin);

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
                return (bar0.unwrap_mem().0, pin as usize);
            }
            Err(e) => panic!("Failed to write BAR0: {:?}", e),
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
                    match (base_class, sub_class, prog_if) {
                        (0x01, 0x08, 0x02) => {
                            info!(
                                "Found NVMe controller at {:02x}:{:02x}.{:x}",
                                address.bus(),
                                address.device(),
                                address.function()
                            );
                            let (base_addr, irq_pin) = nvme_setup(&pci_access, header, address);
                            pci_devices.insert(
                                "nvme0".to_string(),
                                PciDevice::new(
                                    base_addr,
                                    bus,
                                    device,
                                    function,
                                    irq_pin as u8,
                                    PciDeviceType::NVMe,
                                ),
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
