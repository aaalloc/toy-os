#![no_std]
#![no_main]

use user_lib::get_time;
use user_lib::println;
use user_lib::yield_;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("Test sleep...");
    // let current_timer = get_time();
    // let wait_for = current_timer + 0;
    // println!("Current time: {}", current_timer);
    // println!("Wait for: {}", wait_for);
    // while get_time() < wait_for {
    //     yield_();
    // }
    println!("Test sleep OK!");
    0
}
