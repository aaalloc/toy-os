#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{exec, fork, getcwd, waitpid};

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    let mut cwd_buf = [0u8; 256];
    let res = getcwd(&mut cwd_buf, 256);
    println!(
        "Current working directory: {:?}",
        core::str::from_utf8(&cwd_buf[..res as usize])
    );
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    let pid = fork();
                    if pid == 0 {
                        let base = line
                            .split_whitespace()
                            .map(|s| format!("{}\0", s))
                            .collect::<Vec<String>>();
                        let mut args: Vec<*const u8> = base.iter().map(|s| s.as_ptr()).collect();
                        args.push(core::ptr::null());
                        let command = base[0].as_str();
                        if exec(command, args.as_slice()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        // println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
