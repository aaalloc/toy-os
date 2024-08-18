#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[path = "boards/qemu.rs"]
mod board;
mod config;
mod console;
mod drivers;
mod fs;
mod lang_items;
mod logging;
mod memory;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;
use crate::drivers::chardev::UartDevice;
use core::arch::{asm, global_asm};
use drivers::chardev::UART;
use lazy_static::lazy_static;
use sync::UPIntrFreeCell;

global_asm!(include_str!("entry.asm"));

lazy_static! {
    pub static ref DEV_NON_BLOCKING_ACCESS: UPIntrFreeCell<bool> =
        unsafe { UPIntrFreeCell::new(false) };
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}

// thanks to
// https://github.com/rust-embedded/riscv/blob/51fb7736e8003d8500bb356221fd2ba7a43215ba/riscv-rt/src/asm.rs#L190
fn init_fpu() {
    unsafe {
        asm!(
            r#"
        li t0, 0x4000 # bit 14 is FS most significant bit
        li t2, 0x2000 # bit 13 is FS least significant bit
        csrrc x0, sstatus, t0
        csrrs x0, sstatus, t2
        "#
        );
    }
    clear_fpu();
}

#[no_mangle]
pub fn kmain() -> ! {
    clear_bss();
    init_fpu();
    logging::init();

    #[cfg(test)]
    test_main();

    memory::init();
    UART.init();
    task::add_initproc();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    board::device_init();
    *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

fn clear_fpu() {
    unsafe {
        for i in 0..32 {
            asm!("fcvt.d.w f{i}, x0", i = in(reg) i);
            asm!("fmv.d.x f{i}, x0", i = in(reg) i);
            asm!("fmv.w.x f{i}, x0", i = in(reg) i);
        }
    }
}
