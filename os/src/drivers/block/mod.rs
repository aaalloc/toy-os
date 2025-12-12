mod nvme;
mod virtio_blk;
pub use nvme::NVMeController;
pub use virtio_blk::VirtIOBlock;
extern crate alloc;
use alloc::sync::Arc;
use easy_fs::BlockDevice;
use lazy_static::*;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> =
        Arc::new(crate::drivers::block::VirtIOBlock::new());
}
