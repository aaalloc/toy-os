extern crate alloc;
use crate::drivers::chardev::UartDevice;
use crate::drivers::chardev::UART;
use crate::{memory::UserBuffer, print};
use alloc::vec::Vec;

use super::{Dirent, File};

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        let ch = UART.read();
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }

    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }

    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        false
    }

    fn getdents(&self) -> Vec<Dirent> {
        Vec::new()
    }
}

impl File for Stdout {
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }

    fn readable(&self) -> bool {
        false
    }

    fn writable(&self) -> bool {
        true
    }

    fn getdents(&self) -> Vec<Dirent> {
        Vec::new()
    }
}
