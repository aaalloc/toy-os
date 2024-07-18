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
mod loader;
mod memory;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;
mod utils;
use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

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
    trap::init();
    loader::load_apps();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first_task();
    panic!("Unreachable in rust_main!");

    // memory::init();
    // println!("[kernel] back to world!");
    // memory::remap_test();

    // make a pagefault
    panic!("aaaaaaaaaaaaaaaaaaa");
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
