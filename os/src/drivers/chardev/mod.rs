use lazy_static::lazy_static;
mod ns16550a;
pub use ns16550a::NS16550a;
extern crate alloc;
use alloc::sync::Arc;

use crate::{board::UartDeviceImpl, drivers::plic::PlicDevice};

pub trait UartDevice: PlicDevice {
    fn init(&self);
    fn read(&self) -> u8;
    fn write(&self, ch: u8);
}

lazy_static! {
    pub static ref UART: Arc<UartDeviceImpl> = Arc::new(UartDeviceImpl::new());
}
