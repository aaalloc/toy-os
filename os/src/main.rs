#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![feature(slice_ptr_get)]
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
use riscv::register::{
    mcounteren, medeleg, mepc, mhartid, mideleg, mie,
    mstatus::{self, set_mpp, MPP},
    pmpaddr0, pmpcfg0, satp, sie, sstatus, stvec,
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

#[inline(always)]
fn w_stimecmp(value: usize) {
    unsafe {
        asm!(
            r#"
        csrw 0x014d, {}  # stimecmp
        "#,
            in(reg) value
        );
    }
}

#[inline(always)]
fn r_menvcfg() -> usize {
    let value: usize;
    unsafe {
        asm!(
            r#"
        csrr {}, 0x30a  # menvcfg
        "#,
            out(reg) value
        );
    }
    value
}

#[inline(always)]
fn w_menvcfg(value: usize) {
    unsafe {
        asm!(
            r#"
        csrw 0x30a, {}  # menvcfg
        "#,
            in(reg) value
        );
    }
}

#[inline(always)]
fn r_time() -> usize {
    let value: usize;
    unsafe {
        asm!(
            r#"
        csrr {}, 0xc01  # time
        "#,
            out(reg) value
        );
    }
    value
}

#[inline(always)]
fn w_mcounteren(value: usize) {
    unsafe {
        asm!(
            r#"
        csrw mcounteren, {} 
        "#,
            in(reg) value
        );
    }
}

#[inline(always)]
fn r_mcounteren() -> usize {
    let value: usize;
    unsafe {
        asm!(
            r#"
        csrr {}, mcounteren 
        "#,
            out(reg) value
        );
    }
    value
}

fn timerinit() {
    // Enable supervisor-mode timer interrupts
    unsafe {
        mie::set_stimer();
        // enable the sstc extension (i.e. stimecmp).
        w_menvcfg(r_menvcfg() | (1 << 63));

        // allow supervisor to use stimecmp and time.
        w_mcounteren(r_mcounteren() | 2);

        // ask for the very first timer interrupt.
        w_stimecmp(r_time() + 1000000);
    }; // equivalent to w_mie(r_mie() | MIE_STIE)
}

#[inline(always)]
fn r_mstatus() -> usize {
    let value: usize;
    unsafe {
        asm!(
            r#"
        csrr {}, mstatus
        "#,
            out(reg) value
        );
    }
    value
}

#[inline(always)]
fn w_mstatus(value: usize) {
    unsafe {
        asm!(
            r#"
        csrw mstatus, {}
        "#,
            in(reg) value
        );
    }
}

fn write_char(c: u8) {
    let eid = 1; // Example syscall ID for write_char
    let arg0 = c as usize;
    let error: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") eid,
            inlateout("a0") arg0 => error,
        )
    };
    if error != 0 {
        panic!("write_char syscall failed with error code {}", error);
    }
}

#[no_mangle]
pub extern "C" fn start() -> ! {
    unsafe {
        // write_char(b'K');

        // --- Set MPP to Supervisor ---
        let mut x = r_mstatus();
        x &= !(0x3 << 11);
        x |= (1 as usize) << 11;
        w_mstatus(x);

        let sstatus = mstatus::read();
        assert!(
            sstatus.mpp() == MPP::Supervisor,
            "Failed to set MPP to Supervisor mode!"
        );

        mepc::write(kmain as *const () as usize);

        // --- Disable paging temporarily ---
        satp::write(0);

        // --- Delegate all exceptions and interrupts to S-mode ---

        // delegate all exceptions
        // to translate too:
        // w_medeleg(0xffff);
        // w_mideleg(0xffff);
        // w_mcounteren(0xffff);
        // w_scounteren(0xffff);
        // w_sie(r_sie() | SIE_SEIE | SIE_STIE);
        asm!(
            r#"
        li t0, 0xffff
        csrw 0x302, t0  # medeleg
        csrw 0x303, t0  # mideleg
        "#
        );
        // medeleg::set_breakpoint();

        // medeleg::clear_supervisor_env_call();
        // medeleg::clear_load_misaligned();
        // medeleg::clear_store_misaligned();
        // medeleg::clear_illegal_instruction();

        // --- Enable supervisor external and timer interrupts ---
        sie::set_sext();
        sie::set_stimer();

        // --- Configure Physical Memory Protection ---
        pmpaddr0::write(0x3fffffffffffff);
        pmpcfg0::write(0xf);

        // --- Initialize timer ---
        timerinit();

        // --- Store hartid in tp register ---
        let id = mhartid::read();
        core::arch::asm!("mv tp, {}", in(reg) id);

        asm!("csrr t0, sstatus");

        // TODO: why is this causing a trap?
        // write_char(b'K');
        // write_char(b'e');
        // write_char(b'r');
        // write_char(b'n');

        // --- Switch to S-mode and jump to main() ---
        core::arch::asm!("mret");
    }

    // Should never return
    loop {}
}

#[no_mangle]
pub fn kmain() -> ! {
    clear_bss();
    init_fpu();
    logging::init();
    trap::init();
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
        core::slice::from_raw_parts_mut(
            sbss as *const () as usize as *mut u8,
            ebss as *const () as usize - sbss as *const () as usize,
        )
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
