use clap::{App, Arg};
use easy_fs::{BlockDevice, EasyFileSystem, Inode};
use std::fs::{read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::sync::Mutex;

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512).unwrap();
        f
    })));
    // 16MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let bin_inode = root_inode.create_dir("bin").unwrap();
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = bin_inode.create(app.as_str()).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    // list apps
    // for app in root_inode.ls() {
    //     println!("{}", app);
    // }
    Ok(())
}

#[allow(dead_code)]
fn read_string(file: &Arc<Inode>) -> String {
    let mut read_buffer = [0u8; 512];
    let mut offset = 0usize;
    let mut read_str = String::new();
    loop {
        let len = file.read_at(offset, &mut read_buffer);
        if len == 0 {
            break;
        }
        offset += len;
        read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
    }
    read_str
}

#[allow(dead_code)]
fn tree(inode: &Arc<Inode>, name: &str, depth: usize) {
    for _ in 0..depth {
        print!("  ");
    }
    println!("{}", name);
    for name in inode.ls() {
        let child = inode.find(&name).unwrap();
        if child == inode.to_owned() {
            continue;
        }
        tree(&child, &name, depth + 1);
    }
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), 4096, 1);
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);
    root_inode.create("filea");
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(filea.read_at(0, &mut buffer), 0,);
        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SZ);
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    random_str_test(100 * BLOCK_SZ);
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    random_str_test((12 + 128) * BLOCK_SZ);
    random_str_test(400 * BLOCK_SZ);
    random_str_test(1000 * BLOCK_SZ);
    random_str_test(2000 * BLOCK_SZ);

    Ok(())
}

#[test]
fn efs_dir_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), 4096, 1);
    let efs = EasyFileSystem::open(block_file.clone());
    let root = Arc::new(EasyFileSystem::root_inode(&efs));

    let root_parent = root.get_parent();
    assert!(root_parent.is_none());

    root.create("f1");
    root.create("f2");

    let d1 = root.create_dir("d1").unwrap();

    let f3 = d1.create("f3").unwrap();
    let d2 = d1.create_dir("d2").unwrap();
    let f4 = d2.create("f4").unwrap();
    assert_eq!(d2.find(".").unwrap().get_block_id(), d2.get_block_id());
    // assert_eq!(d2.get_parent().unwrap().get_block_id(), d1.get_block_id());
    // assert_eq!(d2.get_name().unwrap(), "d2");
    tree(&root, "/", 0);

    // let d2_parent = d1.get_parent().expect("d2 should have a parent");
    // assert_eq!(d2_parent.get_block_id(), d1.get_block_id());

    // let d2_parent_d1 = d2.get_parent().unwrap();
    // assert_eq!(d2_parent_d1.get_block_id(), d1.get_block_id());
    // let d2_root = d2_parent_d1.get_parent().unwrap();
    // assert_eq!(d2_root.get_block_id(), root.get_block_id());

    let f3_content = "3333333";
    let f4_content = "4444444444444444444";
    f3.write_at(0, f3_content.as_bytes());
    f4.write_at(0, f4_content.as_bytes());

    assert_eq!(read_string(&d1.find("f3").unwrap()), f3_content);
    assert_eq!(read_string(&root.find("/d1/f3").unwrap()), f3_content);
    assert_eq!(read_string(&d2.find("f4").unwrap()), f4_content);
    assert_eq!(read_string(&d1.find("d2/f4").unwrap()), f4_content);
    assert_eq!(read_string(&root.find("/d1/d2/f4").unwrap()), f4_content);
    assert!(f3.find("whatever").is_none());
    Ok(())
}
