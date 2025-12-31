extern crate alloc;
use alloc::sync::Arc;
use fdt::Fdt;

use crate::drivers::{
    block::BlockDeviceManager,
    chardev::{UartDevice, UART},
    plic::{IntrTargetPriority, PlicDevice, PLIC},
};

use spin::Once;

pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x8800_0000;
pub type UartDeviceImpl = crate::drivers::chardev::NS16550a<0x1000_0000>;
pub enum MMIOType {
    Plic,
    Uart,
    VirtioBlk,
    Pci,
}

pub static MMIO_REGIONS: Once<Arc<MMIORegions>> = Once::new();

pub fn find_mmio_regions(fdt: &Fdt) {
    let regions = MMIORegions::collect_mmio_from_fdt(fdt);
    MMIO_REGIONS.call_once(|| Arc::new(regions));
}

pub struct MemoryRegion {
    pub starting_address: usize,
    pub length: usize,
}

pub struct MMIORegions {
    plic: Option<MemoryRegion>,
    uart: Option<MemoryRegion>,
    virtio_blk: Option<MemoryRegion>,
    pci: Option<MemoryRegion>,
}

impl MMIORegions {
    pub fn empty() -> Self {
        MMIORegions {
            plic: None,
            uart: None,
            virtio_blk: None,
            pci: None,
        }
    }

    pub fn get_region(&self, device: MMIOType) -> Option<&MemoryRegion> {
        match device {
            MMIOType::Plic => self.plic.as_ref(),
            MMIOType::Uart => self.uart.as_ref(),
            MMIOType::VirtioBlk => self.virtio_blk.as_ref(),
            MMIOType::Pci => self.pci.as_ref(),
        }
    }

    pub fn add_region(&mut self, device: MMIOType, region: MemoryRegion) {
        match device {
            MMIOType::Plic => self.plic = Some(region),
            MMIOType::Uart => self.uart = Some(region),
            MMIOType::VirtioBlk => self.virtio_blk = Some(region),
            MMIOType::Pci => self.pci = Some(region),
        }
    }

    pub fn get_all_regions(&self) -> alloc::vec::Vec<(MMIOType, &MemoryRegion)> {
        let mut regions = alloc::vec::Vec::new();
        if let Some(region) = &self.plic {
            regions.push((MMIOType::Plic, region));
        }
        if let Some(region) = &self.uart {
            regions.push((MMIOType::Uart, region));
        }
        if let Some(region) = &self.virtio_blk {
            regions.push((MMIOType::VirtioBlk, region));
        }
        if let Some(region) = &self.pci {
            regions.push((MMIOType::Pci, region));
        }
        regions
    }

    pub fn collect_mmio_from_fdt(fdt: &Fdt) -> Self {
        let mut mmio_devices = Self::empty();

        // Example: UART
        if let Some(uart_node) = fdt.find_compatible(&["ns16550a"]) {
            mmio_devices.add_region(
                MMIOType::Uart,
                MemoryRegion {
                    starting_address: uart_node.reg().unwrap().next().unwrap().starting_address
                        as usize,
                    length: uart_node.reg().unwrap().next().unwrap().size.unwrap() as usize,
                },
            );
            log::info!(
                "uart irq number: {}",
                uart_node.interrupts().unwrap().next().unwrap()
            );
        };

        if let Some(node) = fdt.find_compatible(&["virtio,mmio"]) {
            mmio_devices.add_region(
                MMIOType::VirtioBlk,
                MemoryRegion {
                    starting_address: node.reg().unwrap().next().unwrap().starting_address as usize,
                    length: node.reg().unwrap().next().unwrap().size.unwrap() as usize,
                },
            );
        };

        let plic_node = fdt
            .find_compatible(&["riscv,plic0"])
            .or_else(|| fdt.find_compatible(&["sifive,plic-1.0.0"]));

        if let Some(node) = plic_node {
            mmio_devices.add_region(
                MMIOType::Plic,
                MemoryRegion {
                    starting_address: node.reg().unwrap().next().unwrap().starting_address as usize,
                    length: node.reg().unwrap().next().unwrap().size.unwrap() as usize,
                },
            );
        };

        if let Some(pci) = fdt.find_compatible(&["pci-host-ecam-generic"]) {
            mmio_devices.add_region(
                MMIOType::Pci,
                MemoryRegion {
                    starting_address: pci.reg().unwrap().next().unwrap().starting_address as usize,
                    length: pci.reg().unwrap().next().unwrap().size.unwrap() as usize,
                },
            );
        };

        mmio_devices
    }
}

// #[derive(FromRepr, Sequence, Clone, Copy)]
// #[repr(u32)]
// pub enum IrqEnum {
//     // for qemu, normally 0 ??
//     NVME_BLOCK = 2,
//     VIRTIO_BLOCK = 8,
//     // for qemu, 10
//     UART = 7,
// }

static DEVICE_REGISTRY: Once<DeviceRegistry> = Once::new();

#[derive(Default)]
pub struct DeviceRegistry<'a> {
    devices: hashbrown::HashMap<usize, &'a dyn PlicDevice>,
}

impl<'a> DeviceRegistry<'a> {
    pub fn init() {
        DEVICE_REGISTRY.call_once(|| {
            let mut registry = DeviceRegistry::default();
            let block_device = BlockDeviceManager::get();
            registry
                .devices
                .insert(block_device.irq_id(), block_device.as_ref());
            registry.devices.insert(UART.irq_id(), UART.as_ref());
            registry
        });
    }

    pub fn get() -> &'static Self {
        DEVICE_REGISTRY
            .get()
            .expect("Device registry not initialized")
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &&'a dyn PlicDevice)> {
        self.devices.iter()
    }

    pub fn device(&self, irq_id: &usize) -> Option<&'a dyn PlicDevice> {
        self.devices.get(irq_id).copied()
    }
}

pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe {
        PLIC::new(
            MMIO_REGIONS
                .get()
                .unwrap()
                .get_region(MMIOType::Plic)
                .unwrap()
                .starting_address,
        )
    };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;

    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);

    DeviceRegistry::init();
    for (irq_id, _) in DeviceRegistry::get().iter() {
        plic.enable(hart_id, supervisor, *irq_id);
        plic.set_priority(*irq_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn irq_handler() {
    let mut plic = unsafe {
        PLIC::new(
            MMIO_REGIONS
                .get()
                .unwrap()
                .get_region(MMIOType::Plic)
                .unwrap()
                .starting_address,
        )
    };
    let irq_id = plic.claim(0, IntrTargetPriority::Supervisor);
    // match IrqEnum::from_repr(irq_id).expect(alloc::format!("Invalid IRQ {}", irq_id).as_str()) {
    //     IrqEnum::NVME_BLOCK | IrqEnum::VIRTIO_BLOCK => BlockDeviceManager::get().handle_irq(),
    //     IrqEnum::UART => UART.handle_irq(),
    // }
    match DeviceRegistry::get().device(&irq_id.try_into().unwrap()) {
        Some(device_info) => {
            device_info.irq_handler();
            plic.complete(0, IntrTargetPriority::Supervisor, irq_id);
        }
        None => panic!("Unhandled IRQ: {}", irq_id),
    }
}
