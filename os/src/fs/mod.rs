pub mod inode;
pub mod stdio;
extern crate alloc;
use crate::memory::UserBuffer;
use alloc::ffi::CString;
use alloc::vec::Vec;

pub trait File: Send + Sync {
    #[allow(dead_code)]
    fn readable(&self) -> bool;
    #[allow(dead_code)]
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
    fn getdents(&self) -> Vec<Dirent>;
}

#[repr(C)]
pub struct Dirent {
    pub type_: DirentType,
    pub name: CString,
}

impl Dirent {
    pub fn new(name: CString, type_: DirentType) -> Self {
        Self { type_, name }
    }
}

pub enum DirentType {
    File,
    Directory,
}
