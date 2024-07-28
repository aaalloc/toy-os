#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;
use alloc::ffi::CString;

use user_lib::{close, getdents, open, OpenFlags};

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    let path;
    if argc == 1 {
        path = ".";
    } else {
        path = argv[1];
    }
    let fd = open(path, OpenFlags::RDONLY);
    assert_ne!(fd, -1);
    assert!(fd > 0);
    let mut buf = [0u8; 1024];
    let nread = getdents(fd as usize, &mut buf);
    assert_ne!(nread, -1);
    let nread = nread as usize;
    let mut i = 0;
    while i < nread {
        // let t = buf[i];
        let null = buf[i + 1..].iter().position(|&x| x == 0).unwrap();
        let name = CString::new(&buf[i + 1..i + 1 + null]).unwrap();
        print!("{} ", name.to_str().unwrap());
        i += null + 2;
    }
    print!("\n");
    close(fd as usize);
    0
}
