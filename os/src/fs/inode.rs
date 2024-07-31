extern crate alloc;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::lazy_static;

use crate::{
    drivers::block::BLOCK_DEVICE, memory::UserBuffer, task::current_task, utils::UPSafeCell,
};

use super::{Dirent, DirentType, File};

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

pub struct OSInode {
    #[allow(dead_code)]
    readable: bool,
    #[allow(dead_code)]
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

pub fn root_os_inode() -> Arc<OSInode> {
    Arc::new(OSInode::new(true, true, ROOT_INODE.clone()))
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    pub fn get_path(&self) -> CString {
        let inner = self.inner.exclusive_access();
        CString::new(inner.inode.cwd()).unwrap()
    }

    pub fn chdir(&self, path: &str) -> bool {
        let mut inner = self.inner.exclusive_access();
        if let Some(inode) = inner.inode.find(path) {
            inner.inode = inode;
            true
        } else {
            false
        }
    }

    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }

    fn getdents(&self) -> Vec<Dirent> {
        let inner = self.inner.exclusive_access();
        let vec = inner.inode.ls();
        let mut v: Vec<Dirent> = Vec::new();
        for name in vec {
            v.push(Dirent::new(CString::new(name).unwrap(), DirentType::File));
        }
        v
    }
}

pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    // TODO: change ROOT_INODE to current process inode
    let task = current_task();
    let current_os_inode;
    let current_inode;
    // case where we want to first process
    if task.is_none() {
        current_inode = ROOT_INODE.clone();
    } else {
        current_os_inode = task.unwrap().get_cwd_inode();
        current_inode = current_os_inode.inner.exclusive_access().inode.clone();
    }

    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = current_inode.find(name) {
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            current_inode
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        current_inode.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}
