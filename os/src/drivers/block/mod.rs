pub mod nvme;
mod nvme_blk;
mod virtio_blk;

use log::error;
pub use virtio_blk::VirtIOBlock;
extern crate alloc;
use crate::{
    device_tree::DEVICE_TREE_NODES,
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
        let pci_nvme_device = PciRegistry::get()
            .device("nvme0")
            .expect("No NVMe device found");

        let dev: Arc<dyn BlockDeviceTmp> = match NVMeBlock::new(
            pci_nvme_device.get_base_addr(),
            pci_nvme_device.get_irq_id(),
        ) {
            Ok(nvme) => Arc::new(nvme),
            Err(e) => {
                error!("NVMe init failed: {}", e);
                let virtio = DEVICE_TREE_NODES.get().unwrap().get_virtio_blk();
                Arc::new(VirtIOBlock::new(
                    virtio.get_base_addr(),
                    virtio.get_base_addr_size(),
                    virtio.get_irq_id(),
                ))
            }
        };

        BLOCK_DEVICE.call_once(|| dev);
    }

    #[inline]
    pub fn get() -> &'static Arc<dyn BlockDeviceTmp> {
        BLOCK_DEVICE.get().expect("Block device not initialized")
    }
}
