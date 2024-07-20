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
    let time = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
    let current_timer = get_time(&time);
    let wait_for = current_timer + 1000;
    println!("Current time: {}", current_timer);
    println!("Wait for: {}", wait_for);
    while get_time(&time) < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    0
}
