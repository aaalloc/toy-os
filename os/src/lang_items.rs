use core::panic::PanicInfo;

use crate::{println, sbi::shutdown};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    shutdown(true)
}
