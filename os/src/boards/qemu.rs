extern crate alloc;
use enum_iterator::all;
use enum_iterator_derive::Sequence;
use strum_macros::FromRepr;

use crate::drivers::{
    block::BLOCK_DEVICE,
    chardev::{UartDevice, UART},
    plic::{IntrTargetPriority, PLIC},
};

#[allow(non_snake_case, non_upper_case_globals)]
pub mod VirtAddrEnum {
    pub const VIRTTEST: usize = 0x0010_0000;
    pub const UART0: usize = 0x1000_0000;
    pub const VIRTIO: usize = 0x1000_8000;
    pub const PLIC: usize = 0x0C00_0000;
}

pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x8800_0000;
pub type UartDeviceImpl = crate::drivers::chardev::NS16550a<{ VirtAddrEnum::UART0 }>;

pub const MMIO: &[(usize, usize)] = &[
    (VirtAddrEnum::VIRTTEST, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (VirtAddrEnum::VIRTIO, 0x00_1000),   // Virtio Block in virt machine
    (VirtAddrEnum::UART0, 0x100),        // uart0 in virt machine
    (VirtAddrEnum::PLIC, 0x210000),      // PLIC in virt machine
];

#[derive(FromRepr, Sequence, Clone, Copy)]
#[repr(u32)]
pub enum IrqEnum {
    BLOCK = 8,
    UART = 10,
}

pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(VirtAddrEnum::PLIC) };
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
    let mut plic = unsafe { PLIC::new(VirtAddrEnum::PLIC) };
    let irq_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match IrqEnum::from_repr(irq_id).expect(alloc::format!("Invalid IRQ {}", irq_id).as_str()) {
        IrqEnum::BLOCK => BLOCK_DEVICE.handle_irq(),
        IrqEnum::UART => UART.handle_irq(),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, irq_id);
}
pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
