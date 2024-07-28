#![no_std]
#![no_main]

use user_lib::get_time;
use user_lib::println;
use user_lib::yield_;
use user_lib::TimeVal;

extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("Test sleep...");
    let mut time = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
    get_time(&mut time);
    println!("Current time in seconds: {}", time.tv_sec);
    println!("Current time in microseconds: {}", time.tv_usec);
    let wait_for = time.tv_usec as isize + 1000;
    while get_time(&mut time) < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    0
}
