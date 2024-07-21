//! Loading user applications into memory
extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::info;

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get applications data
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static! {
    static ref APP_MAP: Arc<HashMap<&'static str, usize>> = {
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut map = HashMap::new();
        for i in 0..num_app {
            unsafe {
                let mut end = start;
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                map.insert(str, i);
                start = end.add(1);
            }
        }
        Arc::new(map)
    };
}

pub fn get_app_data_from_name(name: &str) -> Option<&'static [u8]> {
    match APP_MAP.get(name) {
        Some(index) => Some(get_app_data(*index)),
        None => {
            info!("App not found: {}", name);
            None
        }
    }
}

pub fn list_apps() {
    info!("List of applications");
    for (name, _) in APP_MAP.iter() {
        info!("* {}", name);
    }
}
