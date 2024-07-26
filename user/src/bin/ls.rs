#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, getdents, open, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let path = "/";
    let fd = open(path, OpenFlags::RDONLY);
    assert_ne!(fd, -1);
    println!("open(\"{}\", OpenFlags::RDONLY) = {}", path, fd);
    assert!(fd > 0);
    let mut buf = [0u8; 1024];
    let nread = getdents(fd as usize, &mut buf);
    for i in 0..nread {
        print!("{}", buf[i as usize] as char);
    }
    println!("getdents({}, &mut buf) = {}", fd, nread);
    assert_ne!(nread, -1);
    close(fd as usize);
    0
}
