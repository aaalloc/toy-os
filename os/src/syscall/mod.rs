use fs::sys_write;
use process::{sys_exit, sys_get_time, sys_yield};

use crate::println;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

mod fs;
mod process;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    // println!(
    // "[kernel] syscall: id = {}, args = [{:#x}, {:#x}, {:#x}]",
    // id, args[0], args[1], args[2]
    // );
    match id {
        SYSCALL_WRITE => {
            sys_write(args[0], args[1] as *const u8, args[2]);
        }
        SYSCALL_EXIT => {
            sys_exit(args[0] as i32);
        }
        SYSCALL_YIELD => {
            sys_yield();
        }
        SYSCALL_GET_TIME => {
            sys_get_time();
        }
        _ => panic!("Unsupported syscall id: {}", id),
    }
    0
}
