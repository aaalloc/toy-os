pub mod nvme;
mod nvme_blk;
mod virtio_blk;
pub use virtio_blk::VirtIOBlock;
extern crate alloc;
use alloc::sync::Arc;
use easy_fs::BlockDevice;
use lazy_static::*;

use crate::drivers::block::nvme_blk::NVMeBlock;

// lazy_static! {
//     pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> =
//         Arc::new(crate::drivers::block::VirtIOBlock::new());
// }
lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = {
        // if detect_nvme() {
        //     Arc::new(NVMeBlock::new())
        // } else {
        // }
        Arc::new(VirtIOBlock::new())
    };
}
