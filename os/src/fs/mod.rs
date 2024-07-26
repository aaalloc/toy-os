pub mod inode;
pub mod stdio;
extern crate alloc;
use crate::memory::UserBuffer;
use alloc::string::String;
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
    pub name: String,
    pub type_: DirentType,
}

impl Dirent {
    pub fn new(name: String, type_: DirentType) -> Self {
        Self { name, type_ }
    }
}

pub enum DirentType {
    File,
    Directory,
}
