use core::arch::asm;

// #[inline(always)]
// fn console_uart_putchar(c: u8) {
//     unsafe {
//         asm!(
//             r#"
//     li t0, 0x10000000   # UART0 base on QEMU virt
//     li t1, 'A'
//     sb t1, 0(t0)
//         "#,
//         );
//     }
//     unsafe {
//         asm!(
//             r#"
//     li t0, 0x10000000   # UART0 base on QEMU virt
//     li t1, '\n'
//     sb t1, 0(t0)
//         "#,
//         );
//     }
// }

#[inline(always)]
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
    // console_uart_putchar(c as u8);
}

pub fn shutdown(failure: bool) -> ! {
    use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};

    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }

    unreachable!()
}

pub fn set_timer(timer: usize) {
    sbi_rt::set_timer(timer as _);
}
