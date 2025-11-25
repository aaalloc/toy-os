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
use log::info;
use riscv::register::{
    mcounteren, medeleg, mepc, mhartid, mideleg, mie,
    mstatus::{self, set_mpp, MPP},
    pmpaddr0, pmpcfg0, satp, sie,
};
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

// unsafe fn delegate_all_traps() {
//     // Delegate all exceptions to S-mode
//     medeleg::set_instruction_misaligned();
//     medeleg::set_instruction_fault();
//     medeleg::set_illegal_instruction();
//     medeleg::set_breakpoint();
//     medeleg::set_load_misaligned();
//     medeleg::set_load_fault();
//     medeleg::set_store_misaligned();
//     medeleg::set_store_fault();
//     medeleg::set_user_env_call();
//     medeleg::set_supervisor_env_call();
//     medeleg::set_instruction_page_fault();
//     medeleg::set_load_page_fault();
//     medeleg::set_store_page_fault();

//     // Delegate all interrupts to S-mode
//     mideleg::set_ssoft();
//     mideleg::set_stimer();
//     mideleg::set_sext();
// }

// pub unsafe fn timerinit() {
//     // Enable supervisor-mode timer interrupts
//     mie::set_stimer(); // equivalent to w_mie(r_mie() | MIE_STIE)

//     // Enable the SSTC extension (bit 63 of menvcfg)
//     let mut value: usize;
//     core::arch::asm!("csrr {}, 0x30a", out(reg) value);
//     value |= 1 << 63;
//     core::arch::asm!("csrw 0x30a, {}", in(reg) value);

//     // Allow supervisor to use stimecmp and time (bit 1 of mcounteren)
//     mcounteren::set_tm();

//     // Ask for the very first timer interrupt
//     timer::set_next_trigger();
// }

// #[no_mangle]
// pub extern "C" fn start() -> ! {
//     unsafe {
//         // --- Set MPP to Supervisor ---
//         set_mpp(MPP::Supervisor);

//         mepc::write(kmain as usize);

//         // --- Disable paging temporarily ---
//         satp::write(0);

//         // --- Delegate all exceptions and interrupts to S-mode ---
//         delegate_all_traps();

//         // --- Enable supervisor external and timer interrupts ---
//         sie::set_stimer();
//         sie::set_sext();

//         // --- Configure Physical Memory Protection ---
//         pmpaddr0::write(0x3fffffffffffff);
//         pmpcfg0::write(0xf);

//         // --- Initialize timer ---
//         timerinit();

//         // --- Store hartid in tp register ---
//         let id = mhartid::read();
//         core::arch::asm!("mv tp, {}", in(reg) id);

//         // --- Switch to S-mode and jump to main() ---
//         core::arch::asm!("mret");
//     }

//     // Should never return
//     loop {}
// }

#[no_mangle]
pub fn kmain() -> ! {
    clear_bss();
    init_fpu();
    logging::init();
    trap::init();
    info!("Kernel initialized!");
    #[cfg(test)]
    test_main();

    memory::init();
    UART.init();
    task::add_initproc();
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
