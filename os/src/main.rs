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
mod lang_items;
mod memory;
mod sbi;
mod utils;
use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));

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

#[no_mangle]
pub fn kmain() -> ! {
    clear_bss();

    #[cfg(test)]
    test_main();

    println!("[kernel] Hello, world!");
    memory::init();
    println!("[kernel] back to world!");
    memory::remap_test();
    panic!("aaaaaaaaaaaaaaaaaaa");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe {
        //
        (a as *mut u8).write_volatile(0)
    });
}
