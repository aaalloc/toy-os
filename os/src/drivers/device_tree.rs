extern crate alloc;
use core::slice::SlicePattern;

use alloc::{sync::Arc, vec::Vec};
use fdt::Fdt;

use crate::drivers::{
    block::BlockDeviceManager,
    chardev::UartDeviceManager,
    plic::{IntrTargetPriority, PlicDevice, PLIC},
};

use spin::Once;

use fdt::node::FdtNode;

pub struct DeviceTreeNode<'a> {
    pub node: Option<FdtNode<'a, 'a>>,
}

pub struct PciNode<'a>(DeviceTreeNode<'a>);

impl<'a> core::ops::Deref for PciNode<'a> {
    type Target = DeviceTreeNode<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> PciNode<'a> {
    pub fn new(node: Option<FdtNode<'a, 'a>>) -> Self {
        Self(DeviceTreeNode::new(node))
    }

    pub fn resolve_pci_irq_id(&self, bus: u8, device: u8, function: u8, irq_pin: u8) -> Option<u8> {
        let node = self.node.as_ref()?;

        let map_data = node.property("interrupt-map")?.value;
        let mask_data = node.property("interrupt-map-mask")?.value;

        let address_cells = u32::from_be_bytes(
            node.property("#address-cells")?
                .value
                .as_slice()
                .try_into()
                .ok()?,
        ) as usize;

        let interrupt_cells = u32::from_be_bytes(
            node.property("#interrupt-cells")?
                .value
                .as_slice()
                .try_into()
                .ok()?,
        ) as usize;

        // Parent interrupt cells (usually 1 for GIC)
        let parent_interrupt_cells = 1usize;

        let mask: Vec<u32> = mask_data
            .chunks_exact(4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .collect();

        let pci_addr = ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8);

        let mut spec = alloc::vec![0u32; address_cells + interrupt_cells];
        spec[0] = pci_addr;
        spec[address_cells] = irq_pin as u32;

        // Apply mask
        for i in 0..spec.len() {
            spec[i] &= mask[i];
        }

        // interrupt-map entry layout:
        // [child specifier][parent phandle][parent interrupt specifier]
        let entry_cells = address_cells + interrupt_cells + 1 + parent_interrupt_cells;
        let entry_bytes = entry_cells * 4;

        for entry in map_data.chunks_exact(entry_bytes) {
            let mut offset = 0;

            // Child specifier
            let child: Vec<u32> = (0..address_cells + interrupt_cells)
                .map(|_| {
                    let v = u32::from_be_bytes(entry[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    v
                })
                .collect();

            // Compare masked spec
            if child == spec {
                // Skip parent phandle
                offset += 4;

                let irq = u32::from_be_bytes(entry[offset..offset + 4].try_into().unwrap());
                return Some(irq as u8);
            }
        }

        None
    }
}

impl<'a> DeviceTreeNode<'a> {
    pub fn new(node: Option<FdtNode<'a, 'a>>) -> Self {
        Self { node }
    }

    pub fn is_valid(&self) -> bool {
        self.node.is_some()
    }

    pub fn get_irq_id(&self) -> usize {
        self.node
            .as_ref()
            .unwrap()
            .interrupts()
            .unwrap()
            .next()
            .unwrap()
    }

    pub fn get_base_addr(&self) -> usize {
        self.node
            .as_ref()
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .starting_address as usize
    }

    pub fn get_base_addr_size(&self) -> usize {
        self.node
            .as_ref()
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .size
            .unwrap() as usize
    }
}

pub struct DeviceTree<'a> {
    uart_node: DeviceTreeNode<'a>,
    plic_node: DeviceTreeNode<'a>,
    virtio_node: DeviceTreeNode<'a>,
    pci_node: PciNode<'a>,
}

impl<'a> DeviceTree<'a> {
    pub fn new(fdt_root: &'a Fdt) -> Self {
        Self {
            virtio_node: DeviceTreeNode::new(fdt_root.find_compatible(&["virtio,mmio"])),
            uart_node: DeviceTreeNode::new(fdt_root.find_compatible(&["ns16550a"])),
            plic_node: DeviceTreeNode::new(
                fdt_root
                    .find_compatible(&["riscv,plic0"])
                    .or_else(|| fdt_root.find_compatible(&["sifive,plic-1.0.0"])),
            ),
            pci_node: PciNode::new(fdt_root.find_compatible(&["pci-host-ecam-generic"])),
        }
    }
    pub fn get_uart(&self) -> &DeviceTreeNode<'a> {
        &self.uart_node
    }

    pub fn get_virtio_blk(&self) -> &DeviceTreeNode<'a> {
        &self.virtio_node
    }

    pub fn get_plic(&self) -> &DeviceTreeNode<'a> {
        &self.plic_node
    }

    pub fn get_pci(&self) -> &PciNode<'a> {
        &self.pci_node
    }

    pub fn as_slice(&self) -> [&DeviceTreeNode<'a>; 4] {
        [
            &self.uart_node,
            &self.plic_node,
            &self.virtio_node,
            &self.pci_node,
        ]
    }
}

pub static DEVICE_TREE_NODES: Once<Arc<DeviceTree>> = Once::new();

pub fn find_mmio_regions(fdt: &'static Fdt) {
    DEVICE_TREE_NODES.call_once(|| Arc::new(DeviceTree::new(fdt)));
}

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

            let uart_device = UartDeviceManager::get();
            registry
                .devices
                .insert(uart_device.irq_id(), uart_device.as_ref());
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
    let mut plic =
        unsafe { PLIC::new(DEVICE_TREE_NODES.get().unwrap().get_plic().get_base_addr()) };
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
    let mut plic =
        unsafe { PLIC::new(DEVICE_TREE_NODES.get().unwrap().get_plic().get_base_addr()) };
    let irq_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match DeviceRegistry::get().device(&irq_id.try_into().unwrap()) {
        Some(device_info) => {
            device_info.irq_handler();
            plic.complete(0, IntrTargetPriority::Supervisor, irq_id);
        }
        None => panic!("Unhandled IRQ: {}", irq_id),
    }
}
