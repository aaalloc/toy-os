pub mod inode;
pub mod stdio;
use crate::memory::UserBuffer;

pub trait File: Send + Sync {
    #[allow(dead_code)]
    fn readable(&self) -> bool;
    #[allow(dead_code)]
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
