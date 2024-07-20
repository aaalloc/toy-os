#![no_std]
#![no_main]

use user_lib::println;

extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    let test = 1.0 + 2.0;
    println!("Test float operation: 1.0 + 2.0 = {}\n", test);
    0
}
