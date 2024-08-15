//! SBI console driver, for text output
use crate::drivers::chardev::UartDevice;
use crate::drivers::chardev::UART;
use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;
struct StdoutKernel;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            UART.write(c as u8);
        }
        Ok(())
    }
}

impl Write for StdoutKernel {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn printk(args: fmt::Arguments) {
    StdoutKernel.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! printk {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::printk(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! printlnk {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::printk(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
/// print string macro
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
/// println string macro
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
