extern crate alloc;
use enum_iterator::all;
use enum_iterator_derive::Sequence;
use fdt::{standard_nodes::MemoryRegion, Fdt};
use strum_macros::FromRepr;

use crate::drivers::{
    block::BLOCK_DEVICE,
    chardev::{UartDevice, UART},
    plic::{IntrTargetPriority, PLIC},
};

pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x8800_0000;
pub type UartDeviceImpl = crate::drivers::chardev::NS16550a<0x1000_0000>;
pub enum MMIODevice {
    Plic,
    Uart,
    Virtio,
    Pci,
}

impl core::fmt::Debug for MMIODevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MMIODevice::Plic => write!(f, "PLIC"),
            MMIODevice::Uart => write!(f, "UART"),
            MMIODevice::Virtio => write!(f, "VIRTIO"),
            MMIODevice::Pci => write!(f, "PCI"),
        }
    }
}

pub struct MMIODevices {
    plic: Option<MemoryRegion>,
    uart: Option<MemoryRegion>,
    virtio: Option<MemoryRegion>,
    pci: Option<MemoryRegion>,
}

impl MMIODevices {
    pub fn empty() -> Self {
        MMIODevices {
            plic: None,
            uart: None,
            virtio: None,
            pci: None,
        }
    }

    pub fn get_region(&self, device: MMIODevice) -> Option<&MemoryRegion> {
        match device {
            MMIODevice::Plic => self.plic.as_ref(),
            MMIODevice::Uart => self.uart.as_ref(),
            MMIODevice::Virtio => self.virtio.as_ref(),
            MMIODevice::Pci => self.pci.as_ref(),
        }
    }

    pub fn add_region(&mut self, device: MMIODevice, region: MemoryRegion) {
        match device {
            MMIODevice::Plic => self.plic = Some(region),
            MMIODevice::Uart => self.uart = Some(region),
            MMIODevice::Virtio => self.virtio = Some(region),
            MMIODevice::Pci => self.pci = Some(region),
        }
    }

    pub fn get_all_regions(&self) -> alloc::vec::Vec<(MMIODevice, &MemoryRegion)> {
        let mut regions = alloc::vec::Vec::new();
        if let Some(region) = &self.plic {
            regions.push((MMIODevice::Plic, region));
        }
        if let Some(region) = &self.uart {
            regions.push((MMIODevice::Uart, region));
        }
        if let Some(region) = &self.virtio {
            regions.push((MMIODevice::Virtio, region));
        }
        if let Some(region) = &self.pci {
            regions.push((MMIODevice::Pci, region));
        }
        regions
    }

    pub fn collect_mmio_from_fdt(fdt: &Fdt) -> Self {
        let mut mmio_devices = Self::empty();

        // Example: UART
        if let Some(uart_node) = fdt.find_compatible(&["ns16550a"]) {
            mmio_devices.add_region(MMIODevice::Uart, uart_node.reg().unwrap().next().unwrap());
        };

        if let Some(node) = fdt.find_compatible(&["virtio,mmio"]) {
            mmio_devices.add_region(MMIODevice::Virtio, node.reg().unwrap().next().unwrap());
        };

        let plic_node = fdt
            .find_compatible(&["riscv,plic0"])
            .or_else(|| fdt.find_compatible(&["sifive,plic-1.0.0"]));

        if let Some(node) = plic_node {
            mmio_devices.add_region(MMIODevice::Plic, node.reg().unwrap().next().unwrap());
        }

        if let Some(pci) = fdt.find_compatible(&["pci-host-ecam-generic"]) {
            mmio_devices.add_region(MMIODevice::Pci, pci.reg().unwrap().next().unwrap());
        };

        mmio_devices
    }
}

#[derive(FromRepr, Sequence, Clone, Copy)]
#[repr(u32)]
pub enum IrqEnum {
    BLOCK = 8,
    UART = 10,
}

pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(0xc000000) };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;

    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);

    for intr_src_id in all::<IrqEnum>() {
        plic.enable(hart_id, supervisor, intr_src_id as usize);
        plic.set_priority(intr_src_id as usize, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(0xc000000) };
    let irq_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match IrqEnum::from_repr(irq_id).expect(alloc::format!("Invalid IRQ {}", irq_id).as_str()) {
        IrqEnum::BLOCK => BLOCK_DEVICE.handle_irq(),
        IrqEnum::UART => UART.handle_irq(),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, irq_id);
}
