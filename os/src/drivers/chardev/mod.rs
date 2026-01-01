mod ns16550a;
pub use ns16550a::NS16550a;
extern crate alloc;
use alloc::sync::Arc;
use spin::Once;

use crate::{device_tree::DEVICE_TREE_NODES, drivers::plic::PlicDevice};

pub trait UartDevice: PlicDevice {
    fn init(&self);
    fn read(&self) -> u8;
    fn write(&self, ch: u8);
}

static UART_DEVICE: Once<Arc<NS16550a>> = Once::new();

pub struct UartDeviceManager;

impl UartDeviceManager {
    pub fn init() {
        let base_addr = DEVICE_TREE_NODES.get().unwrap().get_uart().get_base_addr();
        let irq_id = DEVICE_TREE_NODES.get().unwrap().get_uart().get_irq_id();
        let uart = Arc::new(NS16550a::new(base_addr, irq_id));
        uart.init();
        UART_DEVICE.call_once(|| uart);
    }

    #[inline]
    pub fn get() -> &'static Arc<NS16550a> {
        UART_DEVICE.get().expect("UART device not initialized")
    }
}
