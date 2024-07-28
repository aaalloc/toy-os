//! File and filesystem-related syscalls
extern crate alloc;
use crate::fs::inode::{open_file, OpenFlags};
use crate::fs::{Dirent, DirentType};
use crate::memory::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use alloc::vec::Vec;
use log::{debug, info};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_getdents(fd: usize, buf: *mut u8, buflen: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    let buf = translated_byte_buffer(token, buf, buflen)[0].as_mut_ptr();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        let dirs: Vec<Dirent> = file.getdents();
        let mut offset = 0;
        for dirent in &dirs {
            let dirent_name = dirent.name.as_bytes_with_nul();
            let dirent_name_len = dirent_name.len();
            if offset + dirent_name_len + 1 > buflen {
                break;
            }
            let dirent_type = match dirent.type_ {
                DirentType::File => 0,
                DirentType::Directory => 1,
            };
            unsafe {
                buf.add(offset).write(dirent_type);
                buf.add(offset + 1)
                    .copy_from(dirent_name.as_ptr(), dirent_name_len);
            }
            offset += dirent_name_len + size_of::<DirentType>();
        }
        // return the number of bytes read
        offset as isize
    } else {
        -1
    }
}
