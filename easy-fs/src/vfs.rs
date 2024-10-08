extern crate alloc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::{block_cache_sync_all, get_block_cache},
    block_device::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeType, DIRENT_SZ},
};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl PartialEq for Inode {
    fn eq(&self, other: &Self) -> bool {
        self.block_id == other.block_id && self.block_offset == other.block_offset
    }
}

impl Inode {
    pub fn get_block_id(&self) -> usize {
        self.block_id
    }

    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    pub fn find(&self, path: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        let mut block_id = self.block_id as u32;
        let mut block_offset = self.block_offset;
        if path == "." {
            return Some(Arc::new(Self::new(
                block_id,
                block_offset,
                self.fs.clone(),
                self.block_device.clone(),
            )));
        }
        for name in path.split("/").filter(|s| !s.is_empty()) {
            let inode_id = get_block_cache(block_id as usize, self.block_device.clone())
                .lock()
                .read(block_offset, |disk_inode: &DiskInode| {
                    if disk_inode.is_file() {
                        return None;
                    }
                    self.find_inode_id(name, disk_inode)
                });
            if inode_id.is_none() {
                return None;
            }
            (block_id, block_offset) = fs.get_disk_inode_pos(inode_id.unwrap());
        }
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }
    pub fn is_root(&self) -> bool {
        self.block_id == 2 && self.block_offset == 0
    }

    pub fn get_current_inode_id(&self) -> Option<u32> {
        self.read_disk_inode(|disk_inode| {
            if self.is_root() {
                Some(0)
            } else {
                self.find_inode_id(".", disk_inode)
            }
        })
    }

    pub fn get_name(&self) -> Option<String> {
        // get parent inode, iterate over files and if one matchs current inode return name
        // Note: for folder
        let parent_inode = self.read_disk_inode(|disk_inode| {
            if self.is_root() || disk_inode.is_file() {
                return None;
            }
            self.find_inode_id("..", disk_inode)
        });
        if parent_inode.is_none() {
            if self.is_root() {
                return Some("/".to_string());
            } else {
                return None;
            }
        }
        let current_inode_id = self.get_current_inode_id().unwrap();
        self.get_parent().unwrap().read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut dirent = DirEntry::empty();
            for i in 0..file_count {
                assert_eq!(
                    disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if dirent.name() == "." {
                    continue;
                }
                if dirent.inode_number() == current_inode_id {
                    return Some(ToString::to_string(&dirent.name()));
                }
            }
            None
        })
    }

    pub fn helper_cwd(&self, path: String, inode: &Inode) -> String {
        // recursive function to get the path of inode
        // call self.get_parent() and concat the name of the inode
        if inode.is_root() {
            return alloc::format!("/{}", path);
        }
        let parent = inode.get_parent().expect("parent should exist");
        let name = inode.get_name().expect("name should exist");
        self.helper_cwd(
            if path == "" {
                name
            } else {
                alloc::format!("{}/{}", name, path)
            },
            &parent,
        )
    }

    pub fn cwd(&self) -> String {
        // get the path of current inode
        if self.is_root() {
            return "/".to_string();
        }
        self.helper_cwd("".to_string(), self)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }

    pub fn get_parent(&self) -> Option<Arc<Inode>> {
        // simply read .. folder
        let fs = self.fs.lock();
        let parent_inode_id = self.read_disk_inode(|disk_inode| {
            if self.is_root() || disk_inode.is_file() {
                return None;
            }
            self.find_inode_id("..", disk_inode)
        });
        if parent_inode_id.is_none() {
            return None;
        }
        let (block_id, block_offset) = fs.get_disk_inode_pos(parent_inode_id.unwrap());
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    fn create_inode(&self, name: &str, inode_type: DiskInodeType) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_inode(|root_inode| {
                // assert it is a directory
                assert!(root_inode.is_dir());
                // has the file been created?
                self.find_inode_id(name, root_inode)
            })
            .is_some()
        {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(inode_type);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// Create a folder that has inode pointing to current folder
    pub fn create_dir_link(&self, path: &str, inode: u32) -> Option<Arc<Inode>> {
        if path != "." && path != ".." {
            panic!("path should be . or ..");
        }
        let mut fs = self.fs.lock();

        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(path, inode);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        let (block_id, block_offset) = fs.get_disk_inode_pos(inode);
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// Create a folder that has inode pointing to parent folder
    #[allow(unused)]
    fn create_parent_dir_link(&self) -> Option<Arc<Inode>> {
        todo!()
    }

    /// Create a file in current inode
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        self.create_inode(name, DiskInodeType::File)
    }

    /// Create a directory in current inode
    pub fn create_dir(&self, name: &str) -> Option<Arc<Inode>> {
        let inode = self.create_inode(name, DiskInodeType::Directory);
        if let Some(inode) = &inode {
            let inode_id = self.read_disk_inode(|disk_inode| self.find_inode_id(name, disk_inode));
            inode.create_dir_link(".", inode_id.unwrap());
            inode.create_dir_link("..", self.get_current_inode_id().unwrap());
        }
        inode
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let mut v: Vec<String> = Vec::new();
            if disk_inode.is_file() {
                return v;
            }
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            assert!(disk_inode.is_file());
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            assert!(disk_inode.is_file());
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
