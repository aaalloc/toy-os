use crate::drivers::{
    chardev::{UartDevice, UART},
    plic::{IntrTargetPriority, PLIC},
};

#[allow(non_snake_case, non_upper_case_globals)]
mod VirtAddrEnum {
    pub const VIRTTEST: usize = 0x0010_0000;
    pub const UART0: usize = 0x1000_0000;
    pub const VIRTIO: usize = 0x1000_1000;
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

pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(VirtAddrEnum::PLIC) };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;

    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);

    // irq nums: 10 uart
    for intr_src_id in [10usize] {
        plic.enable(hart_id, supervisor, intr_src_id);
        plic.set_priority(intr_src_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(VirtAddrEnum::PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);

    match intr_src_id {
        10 => UART.handle_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}
pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
