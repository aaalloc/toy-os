// #[inline(always)]
// fn console_uart_putchar(c: usize) {
//     unsafe {
//         asm!(
//             r#"
//             li t0, 0x10000000   # UART0 base
//             mv t1, {c}          # move value from register
//             sb t1, 0(t0)
//             "#,
//             c = in(reg) c
//         );
//     }
// }

#[inline(always)]
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
    // console_uart_putchar(c);
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
