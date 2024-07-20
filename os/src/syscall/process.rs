use crate::println;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_ms;

#[repr(C)]
pub struct TimeVal {
    pub tv_sec: usize,
    pub tv_usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// get time in milliseconds
pub fn sys_get_time(ts: *mut TimeVal) -> isize {
    let t = get_time_ms();
    // this currently hangs the system
    unsafe {
        *ts = TimeVal {
            tv_sec: t / 1_000_000,
            tv_usec: t % 1_000_000,
        };
    }
    0
}

pub fn sys_fork() -> isize {
    todo!();
}

pub fn sys_exec(_path: *const u8) -> isize {
    todo!();
}

pub fn sys_waitpid(_pid: isize, _exit_code: *mut i32) -> isize {
    todo!();
}
