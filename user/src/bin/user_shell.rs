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
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{chdir, exec, fork, getcwd, waitpid};

pub fn get_current_dir() -> String {
    let mut cwd_buf = [0u8; 256];
    let res = getcwd(&mut cwd_buf, 256);
    assert!(res > 0);
    core::str::from_utf8(&cwd_buf[..res as usize])
        .unwrap()
        .to_string()
}

pub fn exec_command(args: Vec<String>) -> i32 {
    let pid = fork();
    if pid == 0 {
        let command = args[0].as_str();
        let mut args: Vec<*const u8> = args.iter().map(|s| s.as_ptr()).collect();
        args.push(core::ptr::null());
        if exec(command, args.as_slice()) == -1 {
            println!("Error when executing!");
            return -4;
        }
        unreachable!();
    } else {
        let mut exit_code: i32 = 0;
        let exit_pid = waitpid(pid as usize, &mut exit_code);
        assert_eq!(pid, exit_pid);
        return exit_code;
        // println!("Shell: Process {} exited with code {}", pid, exit_code);
    }
}

#[no_mangle]
pub fn main() -> i32 {
    let mut line: String = String::new();
    let mut current_dir = get_current_dir();
    print!("{} ", current_dir);
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    let base = line
                        .split_whitespace()
                        .map(|s| format!("{}\0", s))
                        .collect::<Vec<String>>();
                    match base[0].as_str() {
                        "cd\0" => {
                            if base.len() != 2 {
                                println!("Usage: cd <path>");
                                line.clear();
                                print!(">> ");
                                continue;
                            }
                            let path = base[1].as_str();
                            let res = chdir(path);
                            if res == -1 {
                                println!("cd: {}: No such file or directory", path);
                            }
                            current_dir = get_current_dir();
                            line.clear();
                            print!("{} ", current_dir);
                            print!(">> ");
                            continue;
                        }
                        _ => {}
                    }

                    exec_command(base);
                    line.clear();
                }
                print!("{} ", current_dir);
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
