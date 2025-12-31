pub mod nvme;
mod nvme_blk;
mod virtio_blk;

use log::error;
pub use virtio_blk::VirtIOBlock;
extern crate alloc;
use crate::{
    board::{MMIOType, MMIO_REGIONS},
    drivers::{block::nvme_blk::NVMeBlock, pci::PciRegistry, plic::PlicDevice},
};
use alloc::sync::Arc;
use easy_fs::BlockDevice;
use spin::Once;

pub trait BlockDeviceTmp: BlockDevice + PlicDevice {}

static BLOCK_DEVICE: Once<Arc<dyn BlockDeviceTmp>> = Once::new();

pub struct BlockDeviceManager;

impl BlockDeviceManager {
    pub fn init() {
        let (nvme_addr, _) = PciRegistry::get()
            .nvme("nvme0")
            .expect("No NVMe device found");

        let dev: Arc<dyn BlockDeviceTmp> = match NVMeBlock::new(nvme_addr) {
            Ok(nvme) => Arc::new(nvme),
            Err(e) => {
                error!("NVMe init failed: {}", e);
                let virtio = MMIO_REGIONS
                    .get()
                    .expect("MMIO not initialized")
                    .get_region(MMIOType::VirtioBlk)
                    .expect("Virtio MMIO missing");
                Arc::new(VirtIOBlock::new(virtio))
            }
        };

        BLOCK_DEVICE.call_once(|| dev);
    }

    #[inline]
    pub fn get() -> &'static Arc<dyn BlockDeviceTmp> {
        BLOCK_DEVICE.get().expect("Block device not initialized")
    }
}
